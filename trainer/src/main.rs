use std::{
    fs::File,
    io::{BufReader, BufWriter, Read},
    str::FromStr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, AtomicU16, AtomicUsize, Ordering},
        mpsc,
    },
    time::Instant,
};

mod adam;
mod player;
mod scoreboard;
mod simulation;

use chessbb::{ChessGame, GameResult, Side};
use inquire::Select;
use nnue::{Gradient, Network};
use pp0::{Evaluator, STATIC_EVAL, SearchLimit};
use pp0::{MATERIAL_EVAL, MaterialEvaluator};
use rand::{random_bool, random_range};
use std::io::Write;
use termion::{
    async_stdin, clear, cursor,
    raw::{IntoRawMode, RawTerminal},
};

use crate::{
    player::{Epoch, Player, PlayerEvaluator},
    scoreboard::ScoreBoard,
    simulation::{MatchResult, PairResult, play},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum MenuState {
    Train,
    Quit,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum LoopState {
    Train,
    Review,
}

const LEARNING_RATE: f32 = 0.000001; //0.001
const LAMBDA: f32 = 0.1;
const BETA1: f32 = 0.9;
const BETA2: f32 = 0.99;
const MAX_DEPTH_LIMIT: usize = 3;
const BATCH_COUNT: usize = 80000;
const NET_FILENAME: &str = "net.nnue";
const ENM_FILENAME: &str = "enm.nnue";
const LOG_FILENAME: &str = "training.log";
const BOOK: &str = "UHO_Lichess_4852_v1.epd";
const MAX_INSTANCE: u8 = 24;
static INSTANCE_COUNT: AtomicU8 = AtomicU8::new(0);
static EPOCH: AtomicU16 = AtomicU16::new(0);
fn main() -> std::io::Result<()> {
    rayon::ThreadPoolBuilder::new().thread_name(|x: usize| format!("Thread:{x}")).build_global().unwrap();
    let mut net: Network = match prompt_load().prompt().unwrap() {
        true => Network::new(),
        false => {
            let file = File::open(NET_FILENAME)?;
            let mut buf_reader = BufReader::new(file);
            Network::read(&mut buf_reader)?
        }
    };
    let mut state: MenuState = MenuState::Train;
    loop {
        match state {
            MenuState::Quit => {
                if prompt_quit().prompt().unwrap() == true {
                    let mut file = File::create(NET_FILENAME)?;
                    net.write(&mut file)?;
                }
                break;
            }
            MenuState::Train => {
                println!("here1");
                train(&mut net)?;
                state = match prompt_train().prompt().unwrap() {
                    true => MenuState::Train,
                    false => MenuState::Quit,
                };
            }
        }
    }

    Ok(())
}

const LOOP_COUNT_CHECK_LIMIT: usize = 256;
fn train(net: &mut Network) -> std::io::Result<()> {
    println!("here2 {}", net.eval::<true>());

    let mut m: Gradient = Gradient::zeros();
    let mut v: Gradient = Gradient::zeros();

    let log_file = File::create(format!(".log")).unwrap();
    let mut log_file_buff: BufWriter<&File> = BufWriter::new(&log_file);

    let opening_book: Arc<Mutex<Vec<String>>> = {
        let mut opening_book = String::new();
        (File::open(BOOK)?).read_to_string(&mut opening_book)?;
        Arc::new(Mutex::new(opening_book.split('\n').map(|s| s.to_string()).collect()))
    };
    let opening_book_len: usize = { opening_book.lock().unwrap().len() };

    let mut stdout: RawTerminal<std::io::StdoutLock<'static>> = std::io::stdout().lock().into_raw_mode().unwrap();
    let mut stdin: std::io::Bytes<termion::AsyncReader> = async_stdin().bytes();

    let net_ident: String = String::from_str("Net").unwrap();
    let mat_eval_ident: String = String::from_str("Material Eval").unwrap();
    let static_eval_ident: String = String::from_str("Static Eval").unwrap();
    let (mut tx, mut rx) = mpsc::channel::<PairResult>();

    let training_ident: String = format!("Training: {} vs {}", net_ident, mat_eval_ident);
    let review_ident: String = format!("Review: {} vs {}", net_ident, mat_eval_ident);
    let epoch: Epoch = Epoch(EPOCH.load(Ordering::SeqCst));

    let mut batch_size: usize = BATCH_COUNT;
    let mut training_scoreboard: ScoreBoard = ScoreBoard::new(training_ident.clone(), net_ident, mat_eval_ident.clone(), epoch, batch_size);
    let mut review_scoreboard: ScoreBoard = ScoreBoard::new(training_ident, review_ident, mat_eval_ident, epoch, batch_size);
    let mut results: Vec<MatchResult> = Vec::new();

    let mut loop_state: LoopState = LoopState::Train;
    let player1: Player = Player { evaluator: PlayerEvaluator::Network(net.clone()), search_limit: SearchLimit::depth(3) };
    let mut player2: Player = Player { evaluator: PlayerEvaluator::MaterialEvaluator(MATERIAL_EVAL.clone()), search_limit: SearchLimit::depth(1) };

    let start_of_training = Instant::now();
    let mut is_stronger_than_mat_eval: bool = false;
    let mut is_stronger_than_hce_eval: bool = false;
    let mut loop_counter: usize = 0;
    let mut best_win_rate: f32 = 0.0;
    let mut best_lose_rate: f32 = 100.0;

    write!(stdout, "{}", clear::All)?;
    stdout.flush()?;
    loop {
        //listen to 'q' for interupt
        let b = stdin.next();
        if let Some(Ok(b'q')) = b {
            break;
        }

        if loop_counter % LOOP_COUNT_CHECK_LIMIT == 0 {
            let loop_ident: &str = match loop_state {
                LoopState::Train => "training",
                LoopState::Review => "review",
            };
            let scoreboard: &ScoreBoard = match loop_state {
                LoopState::Train => &training_scoreboard,
                LoopState::Review => &review_scoreboard,
            };
            let elapsed = start_of_training.elapsed();
            let samples_per_second = (scoreboard.finished_count as f64) / elapsed.as_secs_f64();
            let samples_left = batch_size.checked_sub(scoreboard.finished_count).unwrap_or(0);
            let eta_seconds_raw = samples_left as f64 / samples_per_second;
            let eta_h = eta_seconds_raw.div_euclid(3600.0);
            let eta_m = eta_seconds_raw.rem_euclid(3600.0).div_euclid(60.0);
            let eta_s = eta_seconds_raw.rem_euclid(60.0);
            write!(stdout, "{}{}", cursor::Goto(1, 1), clear::CurrentLine)?;
            write!(
                stdout,
                "{}Press q to stop. ({} finished: {}/{}, elapsed {}s, eta {}h {}m {:.2}s){}\n\r",
                cursor::Goto(1, 1),
                loop_ident,
                scoreboard.finished_count,
                batch_size,
                elapsed.as_secs(),
                eta_h as isize,
                eta_m as isize,
                eta_s,
                cursor::Goto(1, 14)
            )?;
        }
        //listen to 'q' for interupt

        if INSTANCE_COUNT.load(Ordering::SeqCst) <= MAX_INSTANCE {
            INSTANCE_COUNT.fetch_add(1, Ordering::SeqCst);
            let new_opening_book = opening_book.clone();
            let mut player1 = player1.clone();
            let mut player2 = player2.clone();
            let tx = tx.clone();
            let epoch: Epoch = Epoch(EPOCH.load(Ordering::SeqCst));
            let is_train = loop_state == LoopState::Train;
            let chessgame: ChessGame = ChessGame::start_pos();
            println!("player1: {}", player1.evaluator.eval(&chessgame));
            rayon::spawn(move || {
                //let mut is_tx_ok: bool = true;
                //while is_tx_ok {
                //    let fen = { new_opening_book.lock().unwrap()[random_range(0..opening_book_len)].clone() };
                //    let p1_is_white: bool = random_bool(0.5);
                //    let result1 = match is_train {
                //        true => play::<true>(&mut player1, &mut player2, Some(&fen), p1_is_white),
                //        false => play::<false>(&mut player1, &mut player2, Some(&fen), p1_is_white),
                //    };
                //    let result2 = match is_train {
                //        true => play::<true>(&mut player1, &mut player2, Some(&fen), !p1_is_white),
                //        false => play::<false>(&mut player1, &mut player2, Some(&fen), !p1_is_white),
                //    };
                //    let result: PairResult = PairResult { result1, result2, epoch };
                //    is_tx_ok = tx.send(result).is_ok();
                //}

                INSTANCE_COUNT.fetch_sub(1, Ordering::SeqCst);
            });
        }

        while let Ok(data) = rx.try_recv() {
            if data.epoch == review_scoreboard.epoch {
                review_scoreboard.finished_count += 2;
                match (data.result1.p1_side, data.result1.result) {
                    //net wins
                    (Side::White, GameResult::Win(Side::White)) | (Side::Black, GameResult::Win(Side::Black)) => review_scoreboard.wins += 1,
                    //net losses
                    (Side::White, GameResult::Win(Side::Black)) | (Side::Black, GameResult::Win(Side::White)) => review_scoreboard.losses += 1,
                    //net draws
                    (_, GameResult::Draw) => review_scoreboard.draws += 1,
                }

                match (data.result2.p1_side, data.result2.result) {
                    //net wins
                    (Side::White, GameResult::Win(Side::White)) | (Side::Black, GameResult::Win(Side::Black)) => review_scoreboard.wins += 1,
                    //net losses
                    (Side::White, GameResult::Win(Side::Black)) | (Side::Black, GameResult::Win(Side::White)) => review_scoreboard.losses += 1,
                    //net draws
                    (_, GameResult::Draw) => review_scoreboard.draws += 1,
                }

                if loop_state == LoopState::Train {
                    results.push(data.result1);
                    results.push(data.result2);
                }
            }
        }

        if review_scoreboard.finished_count >= 2 * batch_size {
            let (scoreboard, i) = match loop_state {
                LoopState::Train => (&mut training_scoreboard, 0),
                LoopState::Review => (&mut review_scoreboard, 1),
            };

            scoreboard.write(&mut stdout, (1, 2 + (6 * i)))?;
            scoreboard.write_to_buf(&mut log_file_buff)?;
            log_file_buff.flush()?;
            match loop_state {
                LoopState::Train => {
                    //update net, gradient stuff here
                }
                LoopState::Review => {
                    let new_win_rate: f32 = (scoreboard.wins as f32) / (scoreboard.finished_count as f32);
                    let new_lose_rate: f32 = (scoreboard.losses as f32) / (scoreboard.finished_count as f32);
                    best_win_rate = best_win_rate.max(new_win_rate);
                    best_lose_rate = best_lose_rate.min(new_lose_rate);
                    write!(stdout, "{}{}", cursor::Goto(1, 20), clear::CurrentLine)?;
                    write!(stdout, "{}{}", cursor::Goto(1, 21), clear::CurrentLine)?;
                    write!(stdout, "{}", cursor::Goto(1, 20))?;
                    write!(stdout, "lose rate: {:.2}% (best: {:.2}%)", new_lose_rate * 100.0, best_lose_rate * 100.0,)?;
                    write!(stdout, ", best win rate: {:.2}%\n\r", best_win_rate * 100.0)?;

                    //the MAT_EVAL and HCE_EVAL case
                    if (!is_stronger_than_mat_eval || !is_stronger_than_mat_eval) && best_win_rate >= 0.65 {
                        if let SearchLimit::Depth(d) = player2.search_limit
                            && d.get() < MAX_DEPTH_LIMIT
                        {
                            player2.search_limit = SearchLimit::Depth(d.saturating_add(1));
                        } else {
                            if !is_stronger_than_mat_eval {
                                is_stronger_than_mat_eval = true;
                                player2.evaluator = PlayerEvaluator::StaticEval(STATIC_EVAL);
                                player2.search_limit = SearchLimit::depth(1);
                                scoreboard.p2_identifier = static_eval_ident.clone();
                            }

                            if is_stronger_than_mat_eval && !is_stronger_than_hce_eval {
                                is_stronger_than_hce_eval = true;
                                player2.evaluator = player1.evaluator.clone();
                                player2.search_limit = player1.search_limit.clone();
                                scoreboard.p2_identifier = scoreboard.p1_identifier.clone();
                                if let PlayerEvaluator::Network(enm) = &player2.evaluator {
                                    let mut file = File::create(ENM_FILENAME)?;
                                    enm.write(&mut file)?;
                                }
                            }
                        }
                        best_lose_rate = 1.0;
                        best_win_rate = 0.0;
                    }

                    if (is_stronger_than_mat_eval && is_stronger_than_mat_eval) && best_win_rate >= 0.65 {
                        player2.evaluator = player1.evaluator.clone();
                        player2.search_limit = player1.search_limit.clone();
                        scoreboard.p2_identifier = scoreboard.p1_identifier.clone();
                        best_lose_rate = 1.0;
                        best_win_rate = 0.0;

                        if let PlayerEvaluator::Network(ref enm) = player2.evaluator {
                            let mut file = File::create(ENM_FILENAME)?;
                            enm.write(&mut file)?;
                        }
                    }

                    loop_state = LoopState::Train;
                    //graduation logic here
                }
            }

            scoreboard.finished_count = 0;
            scoreboard.update();
            drop(rx);
            (tx, rx) = mpsc::channel::<PairResult>();
            //scoreboard
        }

        loop_counter = loop_counter.wrapping_add(1);
    }

    Ok(())
}

fn prompt_load<'a>() -> Select<'a, bool> {
    Select::new("New ChessNet?", vec![true, false])
}

fn prompt_train<'a>() -> Select<'a, bool> {
    Select::new("Continue Training?", vec![true, false])
}

fn prompt_quit<'a>() -> Select<'a, bool> {
    Select::new("Save ChessNet?", vec![true, false])
}
