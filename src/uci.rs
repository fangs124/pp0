use std::{
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use chessbb::{Side, TranspositionTable};

use crate::{ChessGame, ChessNet};

const DEBUG: bool = false;
//const foo: usize = size_of::<TranspositionTable>(); //16 bytes
//const bar: usize = size_of::<Mutex<TranspositionTable>>(); //24 bytes
impl ChessNet {
    pub fn uci_loop_start(&mut self) -> io::Result<()> {
        let mut chessgame = ChessGame::start_pos();
        let mut reader = io::BufReader::new(io::stdin());
        let mut buffer = String::with_capacity(1 << 8);
        let mut tt: Arc<Mutex<TranspositionTable>> = Arc::new(Mutex::new(TranspositionTable::new()));

        while let Ok(count) = io::BufRead::read_line(&mut reader, &mut buffer) {
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
                        tt = Arc::new(Mutex::new(TranspositionTable::new()));
                    }
                    "go" => {
                        println!("{}", uci_go(&mut chessgame, cmds.collect::<Vec<&str>>().join(" ").as_str(), self, tt.clone()))
                    }
                    "quit" => return Ok(()),
                    //TODO
                    _ => {} //???
                }
            }
            buffer.clear();
        }
        //loop {}
        Ok(())
    }
}

fn uci_position(chess_game: &mut ChessGame, cmd_str: &str) {
    let mut cmds = cmd_str.split(' ');
    let mut is_parsing_moves = false;
    //println!("cmds: {:?}", cmds);
    while let Some(cmd) = cmds.next() {
        if !is_parsing_moves {
            match cmd {
                "startpos" => *chess_game = ChessGame::start_pos(),
                "FEN" | "fen" => {
                    let mut i = 0;
                    let mut fen: String = String::new();
                    while i < 6 {
                        fen = fen + cmds.next().unwrap() + " ";
                        i += 1;
                    }
                    //let fen = cmds.take(6).fold(String::new(), |a, b| a + " " + b);
                    //println!("fen: {}", fen);
                    *chess_game = ChessGame::from_fen(&fen);

                    if DEBUG {
                        eprintln!("board:\n\r{}", chess_game.cb);
                    }
                    //println!("cmds: {:?}", cmds.clone().collect::<Vec<&str>>());
                    // rnb1kbnr/ppp1pppp/8/4q3/8/2N5/PPPP1PPP/R1BQKBNR w KQkq - 0 1
                }
                "MOVES" | "moves" => {
                    is_parsing_moves = true;
                }
                //TODO
                _ => (),
            }
        } else {
            //example:
            //position fen r1b1kbnr/ppp2ppp/3p1q2/8/2BQP3/8/PPP2PPP/RNB1K2R w KQkq - 0 1 moves e1g1 f6f2 d4f2 g8h6 f2e1
            //eprintln!("cmd: {:?}", cmd);
            chess_game.make_move(cmd);
            //eprintln!("board:\n\r{}", chess_game.cb);
            //for chess_move in chess_game.try_generate_moves().0 {
            //    if chess_move.print_move() == cmd {
            //        //todo: maybe parse into a source/target and do int compare
            //        chess_game.update_state(chess_move);
            //        eprintln!("board:\n\r{}", chess_game.cb);
            //        continue 'a;
            //    }
            //}
        }
    }
}

pub fn uci_go(chess_game: &mut ChessGame, cmd_str: &str, net: &mut ChessNet, tt: Arc<Mutex<TranspositionTable>>) -> String {
    let mut cmds = cmd_str.split(' ');
    let mut wtime: Duration = Duration::from_secs(1);
    let mut btime: Duration = Duration::from_secs(1);
    let mut winc: Duration = Duration::from_secs(0);
    let mut binc: Duration = Duration::from_secs(0);
    let mut depth: Option<usize> = None;
    while let Some(cmd) = cmds.next() {
        match cmd {
            "depth" => depth = Some(cmds.next().unwrap().parse::<usize>().unwrap()),
            "wtime" => wtime = Duration::from_millis(cmds.next().unwrap().parse::<u64>().unwrap_or(600000)),
            "btime" => btime = Duration::from_millis(cmds.next().unwrap().parse::<u64>().unwrap_or(600000)),
            "winc" => winc = Duration::from_millis(cmds.next().unwrap().parse::<u64>().unwrap_or(600000)),
            "binc" => binc = Duration::from_millis(cmds.next().unwrap().parse::<u64>().unwrap_or(600000)),
            _ => (),
        }
    }
    let time_left: Duration = match chess_game.side() {
        Side::White => (wtime / BASE_COEFF) + (winc / INCREMENT_COEFF),
        Side::Black => (btime / BASE_COEFF) + (binc / INCREMENT_COEFF),
    };
    //eprintln!(
    //    "wtime: {}ms, winc: {}ms, btime: {}ms, binc:{}ms",
    //    wtime.as_millis(),
    //    winc.as_millis(),
    //    btime.as_millis(),
    //    binc.as_millis()
    //);
    //search_position(depth)

    let (d, node_count, eval, duration, best_move) = net.iterative_deepening_no_tt(chess_game, depth, time_left, tt);
    let nps = (node_count as f64 / duration.as_secs_f64()) as usize;
    println!("info score cp {eval} depth {d} nodes {} nps {nps} time {}, ", node_count, duration.as_millis());
    return format!("bestmove {}", best_move.print_move());
}

const BASE_COEFF: u32 = 10;
const INCREMENT_COEFF: u32 = 2;
