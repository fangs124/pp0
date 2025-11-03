use chessbb::{GameResult, Side};
use nalgebra::{DVector, SVector};
use nnue::{_Network, Gradient};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::{
    f32::EPSILON,
    os::unix::process,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::Instant,
};
use termion::{
    clear, cursor,
    raw::{IntoRawMode, RawTerminal},
};

use crate::{LAMBDA, LEARNING_RATE, LOOP_COUNT_CHECK_LIMIT, simulation::MatchResult};

pub fn sgd(net: &mut _Network, results: Vec<MatchResult>) {
    let mut i: usize = 0;
    for result in results {
        let grad = game_gradient(net, result);
        let reg = net.regularization_term(LAMBDA);
        net.update_grad(grad + reg, -LEARNING_RATE);

        i += 1;
    }
}

pub fn adam_single_threaded(net: &mut _Network, results: Vec<MatchResult>, beta1: f32, beta2: f32, m: &mut Gradient, v: &mut Gradient) {
    let mut m_grad = m.clone();
    let mut v_grad = v.clone();

    let mut i: usize = 0;
    for result in results {
        let grad = game_gradient(net, result);

        m_grad = beta1 * m_grad + (1.0 - beta1) * grad.clone();
        v_grad = beta2 * v_grad + (1.0 - beta2) * grad.component_square();

        let reg = net.regularization_term(LAMBDA);
        net.update_grad(Gradient::adam(beta1, beta2, i, &m, &v) + reg, -LEARNING_RATE);
        i += 1;
    }
    *m = m_grad;
    *v = v_grad;
}

const GRADIENT_MINIBATCH: usize = 1000;
const MAX_GRADIENT_THREAD_COUNT: usize = 24;
static INSTANCE_COUNT: AtomicU8 = AtomicU8::new(0);
const GRADIENT_LOOP_COUNT_CHECK_LIMIT: usize = 2048 * 4;
pub fn adam(net: &mut _Network, results: Vec<MatchResult>, beta1: f32, beta2: f32, m: &mut Gradient, v: &mut Gradient) -> io::Result<()> {
    let start_of_gradient = Instant::now();
    let total_results = results.len();
    let number_of_updates = total_results / GRADIENT_MINIBATCH;
    let results = Arc::new(Mutex::new(results));
    let next = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicUsize::new(GRADIENT_MINIBATCH.min(total_results)));
    let update_signal: Arc<Mutex<(Gradient, usize)>> = Arc::new(Mutex::new((Gradient::zeros(), usize::MAX)));
    let (tx, rx) = mpsc::channel::<Gradient>();
    let exit_signal = Arc::new(AtomicBool::new(false));
    for _ in 0..MAX_GRADIENT_THREAD_COUNT {
        INSTANCE_COUNT.fetch_add(1, Ordering::SeqCst);
        let results = results.clone();
        let tx = tx.clone();
        let net = net.clone();
        let next = next.clone();
        let stop = stop.clone();
        let mut net = net.clone();
        let mut epoch: usize = 0;
        let exit_signal = exit_signal.clone();
        let update_signal = update_signal.clone();
        rayon::spawn(move || {
            let mut index: usize = 0;
            let mut current_stop: usize = stop.load(Ordering::SeqCst);
            'main: while index < total_results && !exit_signal.load(Ordering::SeqCst) {
                //process gradient
                while index < current_stop {
                    index = next.fetch_add(1, Ordering::SeqCst);
                    if index >= total_results {
                        break 'main;
                    }

                    let result: MatchResult = { results.lock().unwrap()[index].clone() };
                    match tx.send(game_gradient(&mut net, result)) {
                        Ok(()) => (),
                        Err(_) => (),
                    }

                    if index == current_stop {
                        epoch += 1;
                    }
                }

                let update_signal_lock = update_signal.lock().unwrap();
                if update_signal_lock.1 == epoch {
                    let update_gradient = update_signal_lock.0.clone();

                    net.update_grad(update_gradient, -LEARNING_RATE);
                    //index = current_stop;
                    current_stop = stop.load(Ordering::SeqCst);
                }
                drop(update_signal_lock);
                if index >= total_results {
                    break 'main;
                }
            }
            INSTANCE_COUNT.fetch_sub(1, Ordering::SeqCst);
        });
    }

    let mut current_stop: usize = stop.load(Ordering::SeqCst);
    let mut results_processed: usize = 0;
    let mut epoch: usize = 0;
    let mut loop_counter: usize = 0;
    while results_processed < total_results {
        //receive gradients
        let mut total = Gradient::zeros();
        while results_processed < current_stop {
            while let Ok(gradient) = rx.try_recv() {
                total = total + gradient;
                results_processed += 1;
            }

            if loop_counter % GRADIENT_LOOP_COUNT_CHECK_LIMIT == 0 {
                let loop_ident: &str = "gradients";

                //debug
                let next = next.load(Ordering::SeqCst);
                let stop = stop.load(Ordering::SeqCst);
                let instance_count = INSTANCE_COUNT.load(Ordering::SeqCst);

                let elapsed = start_of_gradient.elapsed();
                let gradients_per_second = (results_processed as f64) / elapsed.as_secs_f64();
                let gradients_left = total_results.checked_sub(results_processed).unwrap_or(0);
                let eta_seconds_raw = gradients_left as f64 / gradients_per_second;
                let eta_h = eta_seconds_raw.div_euclid(3600.0);
                let eta_m = eta_seconds_raw.rem_euclid(3600.0).div_euclid(60.0);
                let eta_s = eta_seconds_raw.rem_euclid(60.0);
                let mut stdout: RawTerminal<std::io::StdoutLock<'static>> = std::io::stdout().lock().into_raw_mode().unwrap();
                write!(stdout, "{}{}", cursor::Goto(1, 1), clear::CurrentLine)?;
                write!(
                    stdout,
                    "{}Press q to stop. ({} finished: {}/{}, elapsed {}s, eta {}h {}m {:.2}s) next: {}, stop: {}, epoch: {}, processed: {} total_results: {} instance: {}{}\n\r",
                    cursor::Goto(1, 1),
                    loop_ident,
                    results_processed,
                    total_results,
                    elapsed.as_secs(),
                    eta_h as isize,
                    eta_m as isize,
                    eta_s,
                    next,
                    stop,
                    epoch,
                    results_processed,
                    total_results,
                    instance_count,
                    cursor::Goto(1, 14)
                )?;
                drop(stdout);
            }

            if results_processed == current_stop {
                epoch += 1;
            }

            loop_counter = loop_counter.saturating_add(1);
        }

        //TODO remove this.. this is only for debug
        if loop_counter % GRADIENT_LOOP_COUNT_CHECK_LIMIT == 0 {
            let loop_ident: &str = "gradients";

            //debug
            let next = next.load(Ordering::SeqCst);
            let stop = stop.load(Ordering::SeqCst);
            let instance_count = INSTANCE_COUNT.load(Ordering::SeqCst);

            let elapsed = start_of_gradient.elapsed();
            let gradients_per_second = (results_processed as f64) / elapsed.as_secs_f64();
            let gradients_left = total_results.checked_sub(results_processed).unwrap_or(0);
            let eta_seconds_raw = gradients_left as f64 / gradients_per_second;
            let eta_h = eta_seconds_raw.div_euclid(3600.0);
            let eta_m = eta_seconds_raw.rem_euclid(3600.0).div_euclid(60.0);
            let eta_s = eta_seconds_raw.rem_euclid(60.0);
            let mut stdout: RawTerminal<std::io::StdoutLock<'static>> = std::io::stdout().lock().into_raw_mode().unwrap();
            write!(stdout, "{}{}", cursor::Goto(1, 1), clear::CurrentLine)?;
            write!(
                stdout,
                "{}Press q to stop. ({} finished: {}/{}, elapsed {}s, eta {}h {}m {:.2}s) next: {}, stop: {}, epoch: {}, processed: {} total_results: {} instance: {}{}\n\r",
                cursor::Goto(1, 1),
                loop_ident,
                results_processed,
                total_results,
                elapsed.as_secs(),
                eta_h as isize,
                eta_m as isize,
                eta_s,
                next,
                stop,
                epoch,
                results_processed,
                total_results,
                instance_count,
                cursor::Goto(1, 14)
            )?;
            drop(stdout);
        }
        loop_counter = loop_counter.saturating_add(1);
        //result_processed == current_stop
        //calculate adamw gradient
        total = (1.0 / GRADIENT_MINIBATCH as f32) * total;
        let final_gradient = {
            //TODO: fix these clones
            *m = beta1 * m.clone() + (1.0 - beta1) * total.clone();
            *v = beta2 * v.clone() + (1.0 - beta2) * total.component_square();
            let adam = Gradient::adam(beta1, beta2, epoch, &m, &v);
            (1.0 / number_of_updates as f32) * (adam + net.regularization_term(LAMBDA))
        };

        stop.store(((epoch + 1) * GRADIENT_MINIBATCH).min(total_results), Ordering::SeqCst);
        current_stop = stop.load(Ordering::SeqCst);
        let mut update_signal_lock = update_signal.lock().unwrap();
        *update_signal_lock = (final_gradient.clone(), epoch);
        drop(update_signal_lock);
        net.update_grad(final_gradient, -LEARNING_RATE);
    }
    drop(rx);
    exit_signal.store(true, Ordering::SeqCst);
    Ok(())
}

pub fn game_gradient(net: &mut _Network, data: MatchResult) -> Gradient {
    let pairs = data.pairs.unwrap();
    let total_moves = pairs.len();
    let reward: f32 = match (data.p1_side, data.result) {
        (Side::White, GameResult::Win(Side::White)) | (Side::Black, GameResult::Win(Side::Black)) => 1.0,
        (Side::White, GameResult::Win(Side::Black)) | (Side::Black, GameResult::Win(Side::White)) => -1.0,
        (_, GameResult::Draw) => 0.0,
    };

    let mut total_grad: Gradient = Gradient::zeros();
    let mut ith_move: usize = 0;

    for ((in_stm, in_ntm), eval) in pairs {
        let t: f32 = ith_move as f32 / total_moves as f32;
        let lerp = (1.0 - t.powi(4)).max(0.0) * (eval.min(2000).max(-2000) as f32 / 2000.0) + t.powi(4).min(1.0) * reward;
        let target: DVector<f32> = DVector::from_element(1, lerp);
        //let target: DVector<f32> = DVector::from_element(1, reward);
        let grad = net.backward_prop_sparse(in_stm, in_ntm, target, 1.0);
        ith_move += 1;
        total_grad = total_grad + grad;
    }

    return (1.0 / (total_moves as f32)) * total_grad;
}
