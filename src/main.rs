use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::num::NonZero;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use chessbb::{AtomicTranspositionTable, GameResult, Side};
use inquire::Select;
use rand::random_range;
use termion::raw::IntoRawMode;
use termion::{async_stdin, clear, cursor};

pub use crate::chessgame::ChessGame;
pub use crate::chessnet::ChessNet;
use crate::scoreboard::ScoreBoard;
use crate::simulation::{PlayParameter, TimeControl, TrainingResult, play};
use crate::uci::{uci_go, uci_position};

extern crate chessbb;
extern crate nnet;

mod chessgame;
mod chessnet;
mod scoreboard;
mod simulation;
mod uci;

pub(crate) type GR = GameResult;
pub(crate) type AtomicTT = AtomicTranspositionTable;

const NODE_COUNT: [usize; 3] = [256, 64, 1];
const MAX_INSTANCE: usize = 24;
const BATCH_SIZE: usize = 10000; //~4.8 Mil?
const REVIEW_SIZE: usize = 1000;
const UPDATE_PER_BATCH: usize = 2;

const LEARNING_RATE: f32 = 0.0001;
const FIXED_NODE_LIMIT: NonZero<usize> = NonZero::new(1 << 16).unwrap();
const FALLBACK_DEPTH: usize = 3;
const STUNTED_FALLBACK_DEPTH: usize = 2;

const BASE_TIME: Duration = Duration::from_secs(5);
const INCREMENT_TIME: Duration = Duration::from_millis(5);
const MILLIS_MARGIN: Duration = Duration::from_millis(10);
const BASE_COEFF: u32 = 10;
const INCREMENT_COEFF: u32 = 2;
const USE_TC: bool = false;
const TIME_CONTROL: Option<TimeControl> = match USE_TC {
    true => Some(TimeControl::new(BASE_TIME, INCREMENT_TIME)),
    false => None,
};

static INSTANCE_COUNT: AtomicUsize = AtomicUsize::new(0_usize);
static RETURN_COUNT: AtomicUsize = AtomicUsize::new(0_usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum State {
    Train,
    Quit,
}
const IS_SINGLE_THREADED_MAIN: bool = false;
const IS_ALT: bool = false;
const START_STRONGER_THAN_RAND: bool = false;
const FLIP: bool = true;
const UHO_LICHESS_BOOK: &str = "UHO_Lichess_4852_v1.epd";
const POPULAR_LICHESS_BOOK: &str = "popularpos_lichess_v3.epd"; //FIXME broken!
const IS_UNBALANCED_BOOK: bool = true;
const BOOK: &str = match IS_UNBALANCED_BOOK {
    true => UHO_LICHESS_BOOK,
    false => POPULAR_LICHESS_BOOK,
};
fn alt_main() -> std::io::Result<()> {
    //println!("is_detected: {}", is_x86_feature_detected!("cmpxchg16b"));
    let file = File::open(format!("{:?}net.json", NODE_COUNT))?;
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;
    let mut chessnet: ChessNet = serde_json::from_str(&contents).unwrap();
    if IS_SINGLE_THREADED_MAIN == false {
        chessnet.uci_loop_start()?;
    } else {
        let example_command: String =
            "position fen rnbqkb1r/1p1pnpp1/p1p4p/4p3/2P5/2N1P1P1/PP1PNPBP/R1BQK2R b KQkq - 0 1 moves d7d5 c4d5 c6d5 a1b1 c8f5 e1f1\ngo wtime 17710 btime 14178 winc 5000 binc 5000\n".to_string();
        let mut foo: &[u8] = example_command.as_bytes();
        //identical to uci_loop
        let mut chessgame = ChessGame::start_pos();
        let mut reader = io::BufReader::new(io::stdin());
        let mut buffer = String::with_capacity(1 << 8);
        let mut tt: Arc<AtomicTT> = Arc::new(AtomicTT::new());

        while let Ok(count) = io::BufRead::read_line(&mut foo, &mut buffer) {
            if count == 0 {
                return Ok(());
            }

            let mut cmds = buffer.split_whitespace();
            if let Some(cmd) = cmds.next() {
                match cmd {
                    "isready" => {
                        println!("readyok");
                    }
                    "uci" => {
                        println!("id name pp0");
                        println!("id author Fangs");
                        println!("uciok");
                    }
                    "position" => uci_position(&mut chessgame, cmds.collect::<Vec<&str>>().join(" ").as_str()),
                    "ucinewgame" => {
                        chessgame = ChessGame::start_pos();
                        tt = Arc::new(AtomicTT::new());
                    }
                    "go" => uci_go(&mut chessgame, cmds.collect::<Vec<&str>>().join(" ").as_str(), &mut chessnet, tt.clone()),
                    "quit" => return Ok(()),
                    //TODO
                    _ => {} //???
                }
            }
            buffer.clear();
        }
        //loop {}
    }
    Ok(())
}
//fn main() -> std::io::Result<()> {}
fn main() -> std::io::Result<()> {
    rayon::ThreadPoolBuilder::new().thread_name(|x: usize| format!("Thread:{x}")).build_global().unwrap();
    //unsafe { backtrace_on_stack_overflow::enable() }
    //unsafe { std::env::set_var("RUST_BACKTRACE", "1") };
    if IS_ALT {
        alt_main()?;
        return Ok(());
    }
    let mut is_quit = false;

    //load or new
    let mut chessnet: ChessNet = match prompt_load().prompt().unwrap() {
        true => ChessNet::new(NODE_COUNT.to_vec()),
        false => {
            let file = File::open(format!("{:?}net.json", NODE_COUNT))?;
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
                    let file = File::create(format!("{:?}net.json", NODE_COUNT))?;
                    serde_json::to_writer(file, &chessnet)?;
                }
                is_quit = true;
            }
            State::Train => {
                match IS_SINGLE_THREADED_MAIN {
                    true => {
                        train_st(&mut chessnet)?;
                        return Ok(());
                    }
                    false => train(&mut chessnet)?,
                };
                state = match prompt_train().prompt().unwrap() {
                    true => State::Train,
                    false => State::Quit,
                };
            }
        }
    }

    Ok(())
}

fn train_st(net: &mut ChessNet) -> std::io::Result<()> {
    //let mut game_samples: Vec<(Vec<ChessMove>, Side)> = Vec::new();
    //let mut is_sanity_sample_printed = false;
    let f = File::create(format!("{:?}.log", NODE_COUNT)).unwrap();
    let mut f_buff: BufWriter<&File> = BufWriter::new(&f);

    let mut f_uho_lichess = File::open(BOOK)?;
    let mut s_uho_lichess = String::new();
    f_uho_lichess.read_to_string(&mut s_uho_lichess)?;
    let uho_lichess: Vec<String> = s_uho_lichess.split('\n').map(|s| s.to_string()).collect();
    let uho_lichess_len = uho_lichess.len();
    let mut stream_out = BufWriter::new(std::io::stdout());
    //let mut stdout = std::io::stdout();
    let mut stdout: termion::raw::RawTerminal<std::io::StdoutLock<'static>> = std::io::stdout().lock().into_raw_mode().unwrap();
    let mut stdin: std::io::Bytes<termion::AsyncReader> = async_stdin().bytes();

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

    let mut is_stronger_than_hce = START_STRONGER_THAN_RAND;
    let mut best_win_rate: f32 = 0.0;

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
            let new_net: ChessNet = net.clone();
            let new_enm: Option<ChessNet> = match is_stronger_than_hce {
                true => Some(enm.clone()),
                false => None,
            };
            let new_tx = tx.clone();
            let new_epoch = scoreboard.epoch.clone();
            let fen = uho_lichess[random_range(0..uho_lichess_len)].clone();
            let param = PlayParameter::new(new_epoch, true, Some(fen), TIME_CONTROL);
            //play(new_net, new_enm, None, new_tx, new_epoch, true);
            play(new_net, new_enm, new_tx, &param);
            INSTANCE_COUNT.fetch_sub(1_usize, Ordering::SeqCst);
            RETURN_COUNT.fetch_add(1_usize, Ordering::SeqCst);
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
        if finish_count >= 2 {
            scoreboard.epoch += 1;
            finish_count = 0;
            for training_result in training_results {
                net.process_training_result(training_result);
            }
            training_results = Vec::new();
            batch_count += 1;
        }

        //do io + review
        if batch_count >= 1 {
            net.version += 1;
            batch_count = 0;

            //note: Goto(n,m) -> column n, row m
            //terminal stuff
            write!(stdout, "{}{}{}{}", cursor::Goto(1, 2), clear::CurrentLine, cursor::Goto(1, 3), clear::CurrentLine)?;
            write!(stdout, "{}{}{}{}", cursor::Goto(1, 4), clear::CurrentLine, cursor::Goto(1, 5), clear::CurrentLine)?;
            write!(stdout, "{}{}{}{}", cursor::Goto(1, 6), clear::CurrentLine, cursor::Goto(1, 7), clear::CurrentLine)?;
            write!(stdout, "{}======== training result! ========\n\r", cursor::Goto(1, 2))?;
            write!(stdout, "discarded {}, threads finished: {}", discarded_count, RETURN_COUNT.load(Ordering::SeqCst))?;
            write!(stdout, ", stronger than rand: {}\n\r", is_stronger_than_hce)?;
            scoreboard.write(&mut stdout)?;
            scoreboard.write_to_buf(&mut f_buff)?;
            stream_out.flush()?;
            f_buff.flush()?;
            scoreboard.update();
            //review if net is stronger
            let (tx_r, rx_r) = mpsc::channel::<TrainingResult>();
            let mut review_match_count: usize = 0;
            r_scoreboard.now();
            r_scoreboard.epoch = scoreboard.epoch;
            while review_match_count < 1 {
                //launch a game if there are idle threads
                if INSTANCE_COUNT.load(Ordering::SeqCst) < MAX_INSTANCE {
                    INSTANCE_COUNT.fetch_add(1, Ordering::SeqCst);

                    let new_net: ChessNet = net.clone();
                    let new_enm: Option<ChessNet> = match is_stronger_than_hce && !FLIP {
                        true => Some(enm.clone()),
                        false => None,
                    };
                    let new_tx = tx_r.clone();
                    let new_epoch = r_scoreboard.epoch.clone();
                    let fen = uho_lichess[random_range(0..uho_lichess_len)].clone();
                    let param = PlayParameter::new(new_epoch, false, Some(fen), TIME_CONTROL);
                    //play(new_net, new_enm, None, new_tx, new_epoch, false);
                    play(new_net, new_enm, new_tx, &param);
                    INSTANCE_COUNT.fetch_sub(1_usize, Ordering::SeqCst);
                    review_match_count += 1
                }

                while let Ok(data) = rx_r.try_recv() {
                    if data.epoch == r_scoreboard.epoch {
                        review_match_count += 1;
                        match (data.net_side, data.result) {
                            //net wins
                            (Side::White, GR::WhiteWins) | (Side::Black, GR::BlackWins) => r_scoreboard.wins += 1,
                            //net losses
                            (Side::White, GR::BlackWins) | (Side::Black, GR::WhiteWins) => {
                                //if game_samples.len() <= 5 {
                                //    game_samples.push((data.history.unwrap(), data.net_side))
                                //}
                                r_scoreboard.losses += 1
                            }
                            //net draws
                            (_, GameResult::Draw) => r_scoreboard.draws += 1,
                        }
                    }
                }
            }

            // review games finished
            write!(stdout, "{}{}{}{}", cursor::Goto(1, 8), clear::CurrentLine, cursor::Goto(1, 9), clear::CurrentLine)?;
            write!(stdout, "{}{}{}{}", cursor::Goto(1, 10), clear::CurrentLine, cursor::Goto(1, 11), clear::CurrentLine)?;
            write!(stdout, "{}{}{}{}", cursor::Goto(1, 12), clear::CurrentLine, cursor::Goto(1, 13), clear::CurrentLine)?;

            write!(stdout, "{}======= reviewing net v.{}! =======\n\r", cursor::Goto(1, 8), net.version)?;
            let new_win_rate: f32 = (r_scoreboard.wins as f32) / (review_match_count as f32);
            let new_lose_rate: f32 = (r_scoreboard.losses as f32) / (review_match_count as f32);
            if new_win_rate > best_win_rate {
                best_win_rate = new_win_rate;
                enm = net.clone();
            }
            #[rustfmt::skip]
            write!(stdout, "lose rate: {:.2}% (best: {:.2}%)", new_lose_rate * 100.0, best_lose_rate * 100.0, )?;
            write!(stdout, ", best win rate: {:.2}%\n\r", best_win_rate * 100.0)?;

            if new_lose_rate < best_lose_rate {
                best_lose_rate = new_lose_rate;
                enm = net.clone();
            }

            if !is_stronger_than_hce && best_win_rate > 0.50 && !FLIP {
                is_stronger_than_hce = true;
                best_lose_rate = 100.0;
                best_win_rate = 0.0;
            }

            if FLIP {
                is_stronger_than_hce = !is_stronger_than_hce;
            }

            r_scoreboard.write(&mut stdout)?;
            stream_out.flush()?;
            f_buff.flush()?;
            r_scoreboard.update();

            r_scoreboard.net1_ver = net.version;
            scoreboard.net1_ver = net.version;
            scoreboard.net2_ver = enm.version;

            return Ok(());
        }

        //if !is_sanity_sample_printed && game_samples.len() >= 5 {
        //    let mut game_number = 0;
        //    let f = File::create(format!("gamesamples.log")).unwrap();
        //    let mut f_buff: BufWriter<&File> = BufWriter::new(&f);
        //    for (chessmoves, side) in &game_samples {
        //        write!(f_buff, "game number {game_number}:\n\r")?;
        //        let print_side = match side {
        //            Side::White => "White",
        //            Side::Black => "Black",
        //        };
        //        write!(f_buff, "net_side: {}\n\r", print_side)?;
        //        for chess_move in chessmoves {
        //            writeln!(f_buff, "{}", chess_move.print_move())?;
        //        }
        //        game_number += 1;
        //        //f_buff.flush();
        //    }
        //    is_sanity_sample_printed = true;
        //    //break;
        //}
    }
    write!(stdout, "{}{}", clear::All, cursor::Goto(1, 1))?;
    let enm_file = File::create(format!("{:?}value_enm.json", NODE_COUNT))?;
    serde_json::to_writer(enm_file, &enm)?;
    let net_file = File::create(format!("{:?}value_net.json", NODE_COUNT))?;
    serde_json::to_writer(net_file, &enm)?;
    stdout.flush()?;
    Ok(())
}

fn train(net: &mut ChessNet) -> std::io::Result<()> {
    //let mut game_samples: Vec<(Vec<ChessMove>, Side)> = Vec::new();
    //let mut is_sanity_sample_printed = false;
    let f = File::create(format!("{:?}.log", NODE_COUNT)).unwrap();
    let mut f_buff: BufWriter<&File> = BufWriter::new(&f);

    let mut f_uho_lichess = File::open(BOOK)?;
    let mut s_uho_lichess = String::new();
    f_uho_lichess.read_to_string(&mut s_uho_lichess)?;
    let uho_lichess: Vec<String> = s_uho_lichess.split('\n').map(|s| s.to_string()).collect();
    let uho_lichess_len = uho_lichess.len();
    let mut stream_out = BufWriter::new(std::io::stdout());
    //let mut stdout = std::io::stdout();
    let mut stdout: termion::raw::RawTerminal<std::io::StdoutLock<'static>> = std::io::stdout().lock().into_raw_mode().unwrap();
    let mut stdin: std::io::Bytes<termion::AsyncReader> = async_stdin().bytes();

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

    let mut is_stronger_than_hce = START_STRONGER_THAN_RAND;
    let mut best_win_rate: f32 = 0.0;

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
            let new_net: ChessNet = net.clone();
            let new_enm: Option<ChessNet> = match is_stronger_than_hce {
                true => Some(enm.clone()),
                false => None,
            };
            let new_tx = tx.clone();
            let new_epoch = scoreboard.epoch.clone();
            let fen = uho_lichess[random_range(0..uho_lichess_len)].clone();
            rayon::spawn(move || {
                let param = PlayParameter::new(new_epoch, true, Some(fen), TIME_CONTROL);
                //play(new_net, new_enm, None, new_tx, new_epoch, true);
                play(new_net, new_enm, new_tx, &param);
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
            write!(stdout, "discarded {}, threads finished: {}", discarded_count, RETURN_COUNT.load(Ordering::SeqCst))?;
            write!(stdout, ", stronger than rand: {}\n\r", is_stronger_than_hce)?;
            scoreboard.write(&mut stdout)?;
            scoreboard.write_to_buf(&mut f_buff)?;
            stream_out.flush()?;
            f_buff.flush()?;
            scoreboard.update();
            //review if net is stronger
            let (tx_r, rx_r) = mpsc::channel::<TrainingResult>();
            let mut review_match_count: usize = 0;
            r_scoreboard.now();
            r_scoreboard.epoch = scoreboard.epoch;
            while review_match_count < REVIEW_SIZE {
                //launch a game if there are idle threads
                if INSTANCE_COUNT.load(Ordering::SeqCst) < MAX_INSTANCE {
                    INSTANCE_COUNT.fetch_add(1, Ordering::SeqCst);

                    let new_net: ChessNet = net.clone();
                    let new_enm: Option<ChessNet> = match is_stronger_than_hce && !FLIP {
                        true => Some(enm.clone()),
                        false => None,
                    };
                    let new_tx = tx_r.clone();
                    let new_epoch = r_scoreboard.epoch.clone();
                    let fen = uho_lichess[random_range(0..uho_lichess_len)].clone();
                    rayon::spawn(move || {
                        let param = PlayParameter::new(new_epoch, false, Some(fen), TIME_CONTROL);
                        //play(new_net, new_enm, None, new_tx, new_epoch, false);
                        play(new_net, new_enm, new_tx, &param);
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
                            (Side::White, GR::BlackWins) | (Side::Black, GR::WhiteWins) => {
                                //if game_samples.len() <= 5 {
                                //    game_samples.push((data.history.unwrap(), data.net_side))
                                //}
                                r_scoreboard.losses += 1
                            }
                            //net draws
                            (_, GameResult::Draw) => r_scoreboard.draws += 1,
                        }
                    }
                }
            }

            // review games finished
            write!(stdout, "{}{}{}{}", cursor::Goto(1, 8), clear::CurrentLine, cursor::Goto(1, 9), clear::CurrentLine)?;
            write!(stdout, "{}{}{}{}", cursor::Goto(1, 10), clear::CurrentLine, cursor::Goto(1, 11), clear::CurrentLine)?;
            write!(stdout, "{}{}{}{}", cursor::Goto(1, 12), clear::CurrentLine, cursor::Goto(1, 13), clear::CurrentLine)?;

            write!(stdout, "{}======= reviewing net v.{}! =======\n\r", cursor::Goto(1, 8), net.version)?;
            let new_win_rate: f32 = (r_scoreboard.wins as f32) / (review_match_count as f32);
            let new_lose_rate: f32 = (r_scoreboard.losses as f32) / (review_match_count as f32);
            if new_win_rate > best_win_rate {
                best_win_rate = new_win_rate;
                enm = net.clone();
            }
            #[rustfmt::skip]
            write!(stdout, "lose rate: {:.2}% (best: {:.2}%)", new_lose_rate * 100.0, best_lose_rate * 100.0, )?;
            write!(stdout, ", best win rate: {:.2}%\n\r", best_win_rate * 100.0)?;

            if new_lose_rate < best_lose_rate && !is_stronger_than_hce {
                best_lose_rate = new_lose_rate;
                enm = net.clone();
            }

            if best_win_rate >= 0.55 && is_stronger_than_hce && !FLIP {
                best_lose_rate = new_lose_rate;
                enm = net.clone();
            }

            if !is_stronger_than_hce && best_win_rate > 0.50 && !FLIP {
                is_stronger_than_hce = true;
                best_lose_rate = 100.0;
                best_win_rate = 0.0;
            }

            if FLIP {
                is_stronger_than_hce = !is_stronger_than_hce;
            }

            r_scoreboard.write(&mut stdout)?;
            stream_out.flush()?;
            f_buff.flush()?;
            r_scoreboard.update();

            r_scoreboard.net1_ver = net.version;
            scoreboard.net1_ver = net.version;
            scoreboard.net2_ver = enm.version;

            let enm_file = File::create(format!("{:?}value_enm.json", NODE_COUNT))?;
            serde_json::to_writer(enm_file, &enm)?;
            let net_file = File::create(format!("{:?}value_net.json", NODE_COUNT))?;
            serde_json::to_writer(net_file, &enm)?;
        }

        //if !is_sanity_sample_printed && game_samples.len() >= 5 {
        //    let mut game_number = 0;
        //    let f = File::create(format!("gamesamples.log")).unwrap();
        //    let mut f_buff: BufWriter<&File> = BufWriter::new(&f);
        //    for (chessmoves, side) in &game_samples {
        //        write!(f_buff, "game number {game_number}:\n\r")?;
        //        let print_side = match side {
        //            Side::White => "White",
        //            Side::Black => "Black",
        //        };
        //        write!(f_buff, "net_side: {}\n\r", print_side)?;
        //        for chess_move in chessmoves {
        //            writeln!(f_buff, "{}", chess_move.print_move())?;
        //        }
        //        game_number += 1;
        //        //f_buff.flush();
        //    }
        //    is_sanity_sample_printed = true;
        //    //break;
        //}
    }
    write!(stdout, "{}{}", clear::All, cursor::Goto(1, 1))?;
    let enm_file = File::create(format!("{:?}value_enm.json", NODE_COUNT))?;
    serde_json::to_writer(enm_file, &enm)?;
    let net_file = File::create(format!("{:?}value_net.json", NODE_COUNT))?;
    serde_json::to_writer(net_file, &enm)?;
    stdout.flush()?;
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

//fn train_sanity_test(net: &mut ChessNet) -> std::io::Result<()> {
//    let f = File::create(format!("{:?}.log", NODE_COUNT)).unwrap();
//    let mut f_buff: BufWriter<&File> = BufWriter::new(&f);
//
//    let mut stream_out = BufWriter::new(std::io::stdout());
//    let mut stdout: termion::raw::RawTerminal<std::io::StdoutLock<'static>> =
//        std::io::stdout().lock().into_raw_mode().unwrap();
//    let mut stdin = async_stdin().bytes();
//
//    let mut enm: ChessNet = net.clone();
//
//    //TODO
//    let mut scoreboard: ScoreBoard = ScoreBoard::new(net.version, enm.version);
//    let mut r_scoreboard: ScoreBoard = ScoreBoard::new(net.version, enm.version);
//    let (tx, rx) = mpsc::channel::<TrainingResultSanityTest>();
//
//    write!(stdout, "{}", clear::All)?;
//    stdout.flush()?;
//
//    //* training statistics */
//    let mut discarded_count: usize = 0;
//    let mut training_results: Vec<TrainingResultSanityTest> = Vec::new();
//    let mut finish_count: usize = 0;
//    let mut batch_count: usize = 0;
//    let mut best_lose_rate: f32 = 100.0;
//
//    let mut is_stronger_than_rand = false;
//    let mut best_win_rate_rand: f32 = 0.0;
//
//    let mut sanity_test_chessmoves: Vec<Vec<ChessMove>> = Vec::new();
//    let mut sanity_test_net_side: Vec<Side> = Vec::new();
//    loop {
//        write!(stdout, "{}Press q to stop.{}\n\r", cursor::Goto(1, 1), cursor::Goto(1, 14))?;
//        //listen to 'q' for interupt
//        let b = stdin.next();
//        if let Some(Ok(b'q')) = b {
//            break;
//        }
//
//        //launch a game if there are idle threads
//        if INSTANCE_COUNT.load(Ordering::SeqCst) <= MAX_INSTANCE {
//            INSTANCE_COUNT.fetch_add(1, Ordering::SeqCst);
//            let mut new_net: ChessNet = net.clone();
//            let mut new_enm: ChessNet = enm.clone();
//            let new_tx = tx.clone();
//            let new_epoch = scoreboard.epoch.clone();
//            let new_is_stronger_than_rand = is_stronger_than_rand.clone();
//            rayon::spawn(move || {
//                play_rand_sanity_test(&mut new_net, new_tx, new_epoch);
//                INSTANCE_COUNT.fetch_sub(1_usize, Ordering::SeqCst);
//                RETURN_COUNT.fetch_add(1_usize, Ordering::SeqCst);
//            });
//        }
//
//        //retrieve data
//        while let Ok(data) = rx.try_recv() {
//            if data.epoch == scoreboard.epoch {
//                finish_count += 1;
//                match (data.net_side, data.result) {
//                    //net wins
//                    (Side::White, GR::WhiteWins) | (Side::Black, GR::BlackWins) => scoreboard.wins += 1,
//                    //net losses
//                    (Side::White, GR::BlackWins) | (Side::Black, GR::WhiteWins) => {
//                        scoreboard.losses += 1;
//                        if sanity_test_chessmoves.len() <= 5 {
//                            sanity_test_chessmoves.push(data.chessmoves);
//                            sanity_test_net_side.push(data.net_side);
//                        }
//                    }
//                    //net draws
//                    (_, GameResult::Draw) => scoreboard.draws += 1,
//                }
//                //training_results.push(data);
//            } else {
//                discarded_count += 1;
//            }
//        }
//
//        //update net
//        if finish_count >= BATCH_SIZE {
//            scoreboard.epoch += 1;
//            finish_count = 0;
//            training_results = Vec::new();
//            batch_count += 1;
//        }
//
//        //do io + review
//        if batch_count >= UPDATE_PER_BATCH {
//            net.version += 1;
//            batch_count = 0;
//
//            //note: Goto(n,m) -> column n, row m
//            //terminal stuff
//            write!(stdout, "{}{}{}{}", cursor::Goto(1, 2), clear::CurrentLine, cursor::Goto(1, 3), clear::CurrentLine)?;
//            write!(stdout, "{}{}{}{}", cursor::Goto(1, 4), clear::CurrentLine, cursor::Goto(1, 5), clear::CurrentLine)?;
//            write!(stdout, "{}{}{}{}", cursor::Goto(1, 6), clear::CurrentLine, cursor::Goto(1, 7), clear::CurrentLine)?;
//            write!(stdout, "{}======== training result! ========\n\r", cursor::Goto(1, 2))?;
//            write!(stdout, "discarded {}, threads finished: {}", discarded_count, RETURN_COUNT.load(Ordering::SeqCst))?;
//            write!(stdout, ", stronger than rand: {}\n\r", is_stronger_than_rand)?;
//            scoreboard.write(&mut stdout)?;
//            scoreboard.write_to_buf(&mut f_buff)?;
//            stream_out.flush()?;
//            f_buff.flush()?;
//            scoreboard.update();
//            //review if net is stronger
//            let (tx_r, rx_r) = mpsc::channel::<TrainingResultSanityTest>();
//            let mut review_match_count: usize = 0;
//            r_scoreboard.epoch = scoreboard.epoch;
//            while review_match_count < REVIEW_SIZE {
//                //launch a game if there are idle threads
//                if INSTANCE_COUNT.load(Ordering::SeqCst) < MAX_INSTANCE {
//                    INSTANCE_COUNT.fetch_add(1, Ordering::SeqCst);
//
//                    let mut new_net: ChessNet = net.clone();
//                    let mut new_enm: ChessNet = enm.clone();
//                    let new_tx = tx_r.clone();
//                    let new_epoch = r_scoreboard.epoch.clone();
//                    let is_play_rand = !is_stronger_than_rand;
//                    rayon::spawn(move || {
//                        play_rand_sanity_test(&mut new_net, new_tx, new_epoch);
//                        INSTANCE_COUNT.fetch_sub(1_usize, Ordering::SeqCst);
//                    });
//                }
//
//                while let Ok(data) = rx_r.try_recv() {
//                    if data.epoch == r_scoreboard.epoch {
//                        review_match_count += 1;
//                        match (data.net_side, data.result) {
//                            //net wins
//                            (Side::White, GR::WhiteWins) | (Side::Black, GR::BlackWins) => r_scoreboard.wins += 1,
//                            //net losses
//                            (Side::White, GR::BlackWins) | (Side::Black, GR::WhiteWins) => r_scoreboard.losses += 1,
//
//                            //net draws
//                            (_, GameResult::Draw) => r_scoreboard.draws += 1,
//                        }
//                    }
//                }
//            }
//
//            // review games finished
//            #[rustfmt::skip]
//            write!(stdout, "{}{}{}{}", cursor::Goto(1, 8), clear::CurrentLine, cursor::Goto(1, 9), clear::CurrentLine)?;
//            #[rustfmt::skip]
//            write!(stdout, "{}{}{}{}", cursor::Goto(1,10), clear::CurrentLine, cursor::Goto(1,11), clear::CurrentLine)?;
//            #[rustfmt::skip]
//            write!(stdout, "{}{}{}{}", cursor::Goto(1,12), clear::CurrentLine, cursor::Goto(1,13), clear::CurrentLine)?;
//
//            write!(stdout, "{}======= reviewing net v.{}! =======\n\r", cursor::Goto(1, 8), net.version)?;
//            let new_win_rate: f32 = (r_scoreboard.wins as f32) / (review_match_count as f32);
//            let new_lose_rate: f32 = (r_scoreboard.losses as f32) / (review_match_count as f32);
//            if new_win_rate > best_win_rate_rand {
//                best_win_rate_rand = new_win_rate;
//                enm = net.clone();
//            }
//            #[rustfmt::skip]
//            write!(stdout, "lose rate: {:.2}% (best: {:.2}%)", new_lose_rate * 100.0, best_lose_rate * 100.0, )?;
//            write!(stdout, ", best win rate: {:.2}\n\r", best_win_rate_rand)?;
//
//            if !is_stronger_than_rand && best_win_rate_rand > 0.50 {
//                is_stronger_than_rand = true;
//            }
//
//            if new_lose_rate < best_lose_rate {
//                best_lose_rate = new_lose_rate;
//                enm = net.clone();
//            }
//
//            r_scoreboard.write(&mut stdout)?;
//            stream_out.flush()?;
//            f_buff.flush()?;
//            r_scoreboard.update();
//
//            r_scoreboard.net1_ver = net.version;
//            scoreboard.net1_ver = net.version;
//            scoreboard.net2_ver = enm.version;
//        }
//        if sanity_test_chessmoves.len() >= 5 {
//            let mut game_number = 0;
//            let f = File::create(format!("gamesamples.log")).unwrap();
//            let mut f_buff: BufWriter<&File> = BufWriter::new(&f);
//            for (chessmoves, side) in sanity_test_chessmoves.iter().zip(sanity_test_net_side.iter()) {
//                write!(f_buff, "game number {game_number}:\n\r")?;
//                let print_side = match side {
//                    Side::White => "White",
//                    Side::Black => "Black",
//                };
//                write!(f_buff, "net_side: {}\n\r", print_side)?;
//                for chess_move in chessmoves {
//                    writeln!(f_buff, "{}", chess_move.print_move())?;
//                }
//                game_number += 1;
//                //f_buff.flush();
//            }
//            break;
//        }
//    }
//    write!(stdout, "{}{}", clear::All, cursor::Goto(1, 1))?;
//    stdout.flush()?;
//    *net = enm.clone();
//    Ok(())
//}
