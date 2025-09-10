use std::{
    io,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

use chessbb::{ChessMove, NegamaxData, Side};

use crate::{AtomicTT, ChessGame, ChessNet};

const DEBUG: bool = false;

impl ChessNet {
    pub fn uci_loop_start(&mut self) -> io::Result<()> {
        let mut chessgame = ChessGame::start_pos();
        let mut reader = io::BufReader::new(io::stdin());
        let mut buffer = String::with_capacity(1 << 8);
        let mut tt: Arc<AtomicTT> = Arc::new(AtomicTT::new());

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
                        tt = Arc::new(AtomicTT::new());
                    }
                    "go" => uci_go(&mut chessgame, cmds.collect::<Vec<&str>>().join(" ").as_str(), self, tt.clone()),
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

pub fn uci_position(chess_game: &mut ChessGame, cmd_str: &str) {
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
                        eprintln!("board:\n\r{}", chess_game);
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

pub fn uci_go(chess_game: &mut ChessGame, cmd_str: &str, net: &mut ChessNet, tt: Arc<AtomicTT>) {
    let now: Instant = Instant::now();
    let mut cmds = cmd_str.split(' ');
    let mut wtime: Duration = Duration::from_secs(1);
    let mut btime: Duration = Duration::from_secs(1);
    let mut winc: Duration = Duration::from_secs(0);
    let mut binc: Duration = Duration::from_secs(0);
    let mut max_depth: Option<u16> = None;
    while let Some(cmd) = cmds.next() {
        match cmd {
            "depth" => max_depth = Some(cmds.next().unwrap().parse::<u16>().unwrap()),
            "wtime" => wtime = Duration::from_millis(cmds.next().unwrap().parse::<u64>().unwrap_or(600000)),
            "btime" => btime = Duration::from_millis(cmds.next().unwrap().parse::<u64>().unwrap_or(600000)),
            "winc" => winc = Duration::from_millis(cmds.next().unwrap().parse::<u64>().unwrap_or(600000)),
            "binc" => binc = Duration::from_millis(cmds.next().unwrap().parse::<u64>().unwrap_or(600000)),
            _ => (),
        }
    }
    let time_limit: Duration = match chess_game.side() {
        Side::White => (wtime / BASE_COEFF) + (winc.checked_sub(Duration::from_millis(5)).unwrap_or(Duration::ZERO) / INCREMENT_COEFF),
        Side::Black => (btime / BASE_COEFF) + (binc.checked_sub(Duration::from_millis(5)).unwrap_or(Duration::ZERO) / INCREMENT_COEFF),
    };
    //eprintln!(
    //    "wtime: {}ms, winc: {}ms, btime: {}ms, binc:{}ms",
    //    wtime.as_millis(),
    //    winc.as_millis(),
    //    btime.as_millis(),
    //    binc.as_millis()
    //);
    //search_position(depth)
    uci_iterative_deepening(chess_game, net, max_depth, tt, now, time_limit);
}

pub fn uci_iterative_deepening(chess_game: &mut ChessGame, net: &mut ChessNet, max_depth: Option<u16>, tt: Arc<AtomicTT>, now: Instant, time_limit: Duration) {
    let moves: Vec<ChessMove> = chess_game.try_generate_moves().0;
    assert!(!moves.is_empty());
    let mut node_count: usize = 0;
    let mut best_move: ChessMove = moves[0].clone();

    let (tx, rx) = mpsc::channel::<(ChessMove, i16, usize, u16)>();
    let mut d = 0;
    let max_depth: u16 = match max_depth {
        Some(x) => x,
        None => u16::MAX,
    };

    rayon::spawn(move || {
        let mut d: u16 = 0;
        let mut best_move: ChessMove = best_move;
        let mut duration: Duration = now.elapsed();
        while duration < time_limit && d <= max_depth {
            duration = now.elapsed();
            while let Ok((chess_move_data, eval_data, node_count_data, d_data)) = rx.try_recv() {
                d = d_data + 1;
                best_move = chess_move_data;
                node_count += node_count_data;
                let nps: usize = (node_count as f64 / duration.as_secs_f64()) as usize;
                println!("info score cp {eval_data} depth {d} nodes {} nps {nps} time {} pv {}", node_count_data, duration.as_millis(), best_move.print_move());
            }
        }

        println!("bestmove {}", best_move.print_move());
    });

    'search: while now.elapsed() < time_limit && d <= max_depth {
        let mut best_eval: i16 = i16::MIN + 1;
        for chess_move in &moves {
            if now.elapsed() >= time_limit {
                break 'search;
            }
            let snapshot: chessbb::ChessBoardSnapshot = chess_game.explore_state(chess_move);
            let mut data: NegamaxData = NegamaxData::new_timed(now, time_limit);
            //let mut data: NegamaxData = NegamaxData::new();
            let eval: i16 = -chess_game.negamax(None, Some(-best_eval), d as usize, net, &mut data, tt.clone());
            chess_game.restore_state(snapshot);
            node_count += data.node_count();

            if eval > best_eval {
                best_eval = eval;
                best_move = chess_move.clone();
            }
        }

        if let Err(_) = tx.send((best_move, best_eval, node_count, d)) {
            break;
        }
        //send data
        d += 1;
    }
}

const BASE_COEFF: u32 = 10;
const INCREMENT_COEFF: u32 = 2;
