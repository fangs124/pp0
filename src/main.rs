use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write, stdout};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use chessbb::{GameResult, Side};
use inquire::Select;
use nnet::InputType;
use termion::raw::IntoRawMode;
use termion::{async_stdin, clear, cursor};

use crate::chessnet::{ChessGame, ChessNet};
use crate::scoreboard::ScoreBoard;
use crate::simulation::{TrainingResult, play, review_play};

extern crate chessbb;
extern crate nnet;

mod chessnet;
mod scoreboard;
mod simulation;

type GR = GameResult;

const NODE_COUNT: [usize; 3] = [128, 64, 1];
const MAX_INSTANCE: usize = 24;
const BATCH_SIZE: usize = 1000;
const REVIEW_SIZE: usize = 1000;
const UPDATE_PER_BATCH: usize = 1;

const LEARNING_RATE: f32 = 0.01;
const FALLBACK_DEPTH: usize = 2;

static INSTANCE_COUNT: AtomicUsize = AtomicUsize::new(0_usize);
static RETURN_COUNT: AtomicUsize = AtomicUsize::new(0_usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum State {
    Train,
    Quit,
}

fn main() -> std::io::Result<()> {
    let mut is_quit = false;

    //load or new
    let mut chessnet: ChessNet = match prompt_load().prompt().unwrap() {
        true => ChessNet::new(NODE_COUNT.to_vec()),
        false => {
            let file = File::open(format!("{:?}value_net.json", NODE_COUNT))?;
            let mut buf_reader = BufReader::new(file);
            let mut contents = String::new();
            buf_reader.read_to_string(&mut contents)?;
            serde_json::from_str(&contents).unwrap()
        }
    };

    let mut state: State = State::Train;
    while is_quit == false {
        match state {
            State::Quit => {
                if prompt_quit().prompt().unwrap() == true {
                    let file = File::create(format!("{:?}value_net.json", NODE_COUNT))?;
                    serde_json::to_writer(file, &chessnet)?;
                }
                is_quit = true;
            }
            State::Train => {
                train(&mut chessnet)?;
                state = match prompt_train().prompt().unwrap() {
                    true => State::Train,
                    false => State::Quit,
                };
            }
        }
    }

    Ok(())
}

fn train(net: &mut ChessNet) -> std::io::Result<()> {
    let f = File::create(format!("{:?}.log", NODE_COUNT)).unwrap();
    let mut f_buff: BufWriter<&File> = BufWriter::new(&f);

    let mut stream_out = BufWriter::new(std::io::stdout());
    let mut stdout: termion::raw::RawTerminal<std::io::StdoutLock<'static>> =
        std::io::stdout().lock().into_raw_mode().unwrap();
    let mut stdin = async_stdin().bytes();

    let mut enm: ChessNet = net.clone();

    //TODO
    let mut scoreboard: ScoreBoard = ScoreBoard::new(net.version, enm.version);
    let mut r_scoreboard: ScoreBoard = ScoreBoard::new(net.version, enm.version);
    let (tx, rx) = mpsc::channel::<TrainingResult>();

    write!(stdout, "{}", clear::All)?;
    stdout.flush()?;

    //* training statistics */
    let mut discarded_count: usize = 0;
    let mut training_results: Vec<TrainingResult> = Vec::new();
    let mut finish_count: usize = 0;
    let mut batch_count: usize = 0;
    let mut best_lose_rate: f32 = 100.0;
    loop {
        write!(stdout, "{}Press q to stop.{}\n\r", cursor::Goto(1, 1), cursor::Goto(1, 14))?;
        //listen to 'q' for interupt
        let b = stdin.next();
        if let Some(Ok(b'q')) = b {
            break;
        }

        //launch a game if there are idle threads
        if INSTANCE_COUNT.load(Ordering::SeqCst) <= MAX_INSTANCE {
            INSTANCE_COUNT.fetch_add(1, Ordering::SeqCst);

            let mut new_net: ChessNet = net.clone();
            let mut new_enm: ChessNet = enm.clone();
            let new_tx = tx.clone();
            let new_epoch = scoreboard.epoch.clone();

            rayon::spawn(move || {
                //TODO
                play(&mut new_net, &mut new_enm, new_tx, new_epoch);
                INSTANCE_COUNT.fetch_sub(1_usize, Ordering::SeqCst);
                RETURN_COUNT.fetch_add(1_usize, Ordering::SeqCst);
            });
        }

        //retrieve data
        while let Ok(data) = rx.try_recv() {
            if data.epoch == scoreboard.epoch {
                finish_count += 1;
                match (data.net_side, data.result) {
                    //net wins
                    (Side::White, GR::WhiteWins) | (Side::Black, GR::BlackWins) => scoreboard.wins += 1,
                    //net losses
                    (Side::White, GR::BlackWins) | (Side::Black, GR::WhiteWins) => scoreboard.losses += 1,
                    //net draws
                    (_, GameResult::Draw) => scoreboard.draws += 1,
                }
                training_results.push(data);
            } else {
                discarded_count += 1;
            }
        }

        //update net
        if finish_count >= BATCH_SIZE {
            scoreboard.epoch += 1;
            finish_count = 0;
            for training_result in training_results {
                net.process_training_result(training_result);
            }
            training_results = Vec::new();
            batch_count += 1;
        }

        //do io + review
        if batch_count >= UPDATE_PER_BATCH {
            net.version += 1;
            batch_count = 0;

            //note: Goto(n,m) -> column n, row m
            //terminal stuff
            write!(stdout, "{}{}{}{}", cursor::Goto(1, 2), clear::CurrentLine, cursor::Goto(1, 3), clear::CurrentLine)?;
            write!(stdout, "{}{}{}{}", cursor::Goto(1, 4), clear::CurrentLine, cursor::Goto(1, 5), clear::CurrentLine)?;
            write!(stdout, "{}{}{}{}", cursor::Goto(1, 6), clear::CurrentLine, cursor::Goto(1, 7), clear::CurrentLine)?;
            write!(stdout, "{}======== training result! ========\n\r", cursor::Goto(1, 2))?;
            #[rustfmt::skip]
            write!(stdout, "discarded {}, threads finished: {}\n\r", discarded_count, RETURN_COUNT.load(Ordering::SeqCst))?;
            scoreboard.write(&mut stdout)?;
            scoreboard.write_to_buf(&mut f_buff)?;
            stream_out.flush()?;
            f_buff.flush()?;
            scoreboard.update();

            //review if net is stronger
            let (tx_r, rx_r) = mpsc::channel::<TrainingResult>();
            let mut review_match_count: usize = 0;
            r_scoreboard.epoch = scoreboard.epoch;
            while review_match_count < REVIEW_SIZE {
                //launch a game if there are idle threads
                if INSTANCE_COUNT.load(Ordering::SeqCst) < MAX_INSTANCE {
                    INSTANCE_COUNT.fetch_add(1, Ordering::SeqCst);

                    let mut new_net: ChessNet = net.clone();
                    let mut new_enm: ChessNet = enm.clone();
                    let new_tx = tx_r.clone();
                    let new_epoch = r_scoreboard.epoch.clone();

                    rayon::spawn(move || {
                        review_play(&mut new_net, &mut new_enm, new_tx, new_epoch);
                        INSTANCE_COUNT.fetch_sub(1_usize, Ordering::SeqCst);
                    });
                }

                while let Ok(data) = rx_r.try_recv() {
                    if data.epoch == r_scoreboard.epoch {
                        review_match_count += 1;
                        match (data.net_side, data.result) {
                            //net wins
                            (Side::White, GR::WhiteWins) | (Side::Black, GR::BlackWins) => r_scoreboard.wins += 1,
                            //net losses
                            (Side::White, GR::BlackWins) | (Side::Black, GR::WhiteWins) => r_scoreboard.losses += 1,
                            //net draws
                            (_, GameResult::Draw) => r_scoreboard.draws += 1,
                        }
                    }
                }
            }

            // review games finished
            #[rustfmt::skip]
            write!(stdout, "{}{}{}{}", cursor::Goto(1, 8), clear::CurrentLine, cursor::Goto(1, 9), clear::CurrentLine)?;
            #[rustfmt::skip]
            write!(stdout, "{}{}{}{}", cursor::Goto(1,10), clear::CurrentLine, cursor::Goto(1,11), clear::CurrentLine)?;
            #[rustfmt::skip]
            write!(stdout, "{}{}{}{}", cursor::Goto(1,12), clear::CurrentLine, cursor::Goto(1,13), clear::CurrentLine)?;

            write!(stdout, "{}======= reviewing net v.{}! =======\n\r", cursor::Goto(1, 8), net.version)?;
            let new_net_lose_rate = (r_scoreboard.losses as f32) / (review_match_count as f32);
            #[rustfmt::skip]
            write!(stdout, "new lose rate: {:.2}%, best lose rate: {:.2}%\n\r", new_net_lose_rate * 100.0, best_lose_rate * 100.0)?;

            if new_net_lose_rate < best_lose_rate {
                best_lose_rate = new_net_lose_rate;
                enm = net.clone();
            }

            r_scoreboard.write(&mut stdout)?;
            stream_out.flush()?;
            f_buff.flush()?;
            r_scoreboard.update();

            r_scoreboard.net1_ver = net.version;
            scoreboard.net1_ver = net.version;
            scoreboard.net2_ver = enm.version;
        }
    }
    write!(stdout, "{}{}", clear::All, cursor::Goto(1, 1))?;
    stdout.flush()?;
    *net = enm.clone();
    Ok(())
}

fn prompt_load<'a>() -> Select<'a, bool> {
    Select::new("New Network?", vec![true, false])
}

fn prompt_train<'a>() -> Select<'a, bool> {
    Select::new("Continue Training?", vec![true, false])
}

fn prompt_quit<'a>() -> Select<'a, bool> {
    Select::new("Save Network?", vec![true, false])
}
