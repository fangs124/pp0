use std::{
    io,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

use chessbb::{ChessGame, ChessMove, Side, Square};
use nnue::Network;

use crate::{
    Evaluator, SearchData, SearchLimit, TimeLimit, TranspositionTable,
    search::{self, WIN_SCORE},
};
type TT = TranspositionTable;

pub fn uci_loop(net: &mut Network) -> io::Result<()> {
    let mut chessgame = ChessGame::start_pos();
    let mut reader = io::BufReader::new(io::stdin());
    let mut buffer = String::with_capacity(1 << 8);
    let mut tt: Arc<TT> = Arc::new(TT::new());
    let mut last_fen = Vec::<String>::new();
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
                "position" => uci_position(&mut chessgame, cmds.collect::<Vec<&str>>().join(" ").as_str(), &mut last_fen),
                "ucinewgame" => {
                    chessgame = ChessGame::start_pos();
                    tt = Arc::new(TT::new());
                    last_fen = Vec::<String>::new();
                }
                "go" => uci_go(&mut chessgame, cmds.collect::<Vec<&str>>().join(" ").as_str(), net, tt.clone()),
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

pub fn uci_position(chessgame: &mut ChessGame, cmd_str: &str, last_fen: &mut Vec<String>) {
    let mut cmds = cmd_str.split(' ');
    let mut is_parsing_moves = false;
    //println!("cmds: {:?}", cmds);
    while let Some(cmd) = cmds.next() {
        if !is_parsing_moves {
            match cmd {
                "startpos" => *chessgame = ChessGame::start_pos(),
                "FEN" | "fen" => {
                    let mut i = 0;
                    let mut fen: String = String::new();
                    while i < 6 {
                        fen = fen + cmds.next().unwrap() + " ";
                        i += 1;
                    }
                    //let fen = cmds.take(6).fold(String::new(), |a, b| a + " " + b);
                    //println!("fen: {}", fen);
                    *chessgame = ChessGame::from_fen(&fen);

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
            chessgame.update_state(&chessgame.parse_move(cmd));
        }
    }
}

pub fn uci_go(chessgame: &mut ChessGame, cmd_str: &str, net: &mut Network, tt: Arc<TT>) {
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
    let mut hard_time_limit: Duration = match chessgame.side() {
        Side::White => (wtime / HARD_BASE_COEFF) + (winc / HARD_INCREMENT_COEFF),
        Side::Black => (btime / HARD_BASE_COEFF) + (binc / HARD_INCREMENT_COEFF),
    };
    hard_time_limit = hard_time_limit.checked_sub(MILLIS_MARGIN).unwrap_or(hard_time_limit);

    let mut soft_time_limit: Duration = match chessgame.side() {
        Side::White => (wtime / SOFT_BASE_COEFF) + (winc / SOFT_INCREMENT_COEFF),
        Side::Black => (btime / SOFT_BASE_COEFF) + (binc / SOFT_INCREMENT_COEFF),
    };
    //soft_time_limit = soft_time_limit.checked_sub(MILLIS_MARGIN).unwrap_or(soft_time_limit);
    //eprintln!(
    //    "wtime: {}ms, winc: {}ms, btime: {}ms, binc:{}ms",
    //    wtime.as_millis(),
    //    winc.as_millis(),
    //    btime.as_millis(),
    //    binc.as_millis()
    //);
    //search_position(depth)
    //let mut node_count: usize = 0;
    //let chess_moves: Vec<ChessMove> = chess_game.try_generate_moves().0;
    //let (eval, best_move) = chess_game.find_move(net, 4, &mut node_count, chess_moves, tt, None);
    //let duration = now.elapsed();
    //let nps = ((node_count as f64) / (duration.as_secs_f64())) as usize;
    //println!("info score cp {eval} depth 4 nodes {node_count} nps {nps} time {} pv {}", duration.as_millis(), best_move.print_move());
    //println!("bestmove {}", best_move.print_move());
    uci_iterative_deepening(chessgame, net, max_depth, tt, now, soft_time_limit, hard_time_limit);
}

const LOOP_COUNT_CHECK_LIMIT: usize = 2048;
pub fn uci_iterative_deepening(
    chessgame: &mut ChessGame, net: &mut Network, max_depth: Option<u16>, tt: Arc<TT>, now: Instant, soft_time_limit: Duration, hard_time_limit: Duration,
) {
    let moves = chessgame.try_generate_moves().0;
    assert!(!moves.is_empty());

    let mut node_count: usize = 0;
    let mut best_move: ChessMove = moves[0].clone();
    if moves.len() == 1 {
        println!("bestmove {}", best_move.print_move());
        return;
    }
    let mut best_d: u16 = 0;
    let (tx, rx) = mpsc::channel::<(ChessMove, i16, usize, u16)>();
    let mut d = 1;
    let max_depth: u16 = match max_depth {
        Some(x) => x,
        None => u8::MAX as u16,
    };
    let tt_new = tt.clone();

    rayon::spawn(move || {
        let mut d: u16 = 1;
        let mut best_move: ChessMove = best_move;
        let mut duration: Duration = now.elapsed();
        while duration < hard_time_limit && d <= max_depth {
            duration = now.elapsed();
            if let Ok((chess_move_data, eval_data, node_count_data, d_data)) = rx.try_recv() {
                let hashfull_count_permill: usize = tt_new.permil_count();
                d = d_data;
                best_move = chess_move_data;
                node_count += node_count_data;
                let nps: usize = (node_count as f64 / duration.as_secs_f64()) as usize;
                let mating_ply: i16 = ((eval_data.signum() * WIN_SCORE - eval_data) / 2) + 1;
                if mating_ply.abs() < 32 && eval_data != 0 {
                    println!(
                        "info score mate {mating_ply} depth {d} nodes {} nps {nps} time {} pv {} hashfull {}",
                        node_count_data,
                        duration.as_millis(),
                        best_move.print_move(),
                        hashfull_count_permill
                    );
                } else {
                    println!(
                        "info score cp {eval_data} depth {d} nodes {} nps {nps} time {} pv {} hashfull {}",
                        node_count_data,
                        duration.as_millis(),
                        best_move.print_move(),
                        hashfull_count_permill
                    );
                }
            }
        }

        println!("bestmove {}", best_move.print_move());
    });

    let mut nodes_since_last_check: usize = 0;
    let mut search_data: SearchData = SearchData::new();
    //search_data.find_move(chess_game, net, tt.clone(), &time_limit, moves);

    'search: while now.elapsed() < soft_time_limit && d <= max_depth {
        let mut best_eval: i16 = i16::MIN + 1;
        net.update(&chessgame, &best_move);
        let snapshot: chessbb::ChessBoardSnapshot = chessgame.explore_state(&best_move);
        let eval: i16 =
            -search_data.negamax::<true, false>(chessgame, best_eval, i16::MAX - 1, d as usize, net, tt.clone(), Some((now, hard_time_limit)), None);
        chessgame.restore_state(snapshot);
        net.revert(&chessgame, &best_move);

        if search_data.is_aborted() {
            break 'search;
        }

        if eval > best_eval || d > best_d {
            best_eval = eval;
            best_d = d;
        }

        if now.elapsed() >= soft_time_limit {
            break 'search;
        }

        //search previous best_move
        for &chessmove in moves.iter() {
            if chessmove == best_move {
                continue;
            }

            net.update(&chessgame, &chessmove);
            let snapshot: chessbb::ChessBoardSnapshot = chessgame.explore_state(&chessmove);
            let eval: i16 =
                -search_data.negamax::<true, false>(chessgame, best_eval, i16::MAX - 1, d as usize, net, tt.clone(), Some((now, hard_time_limit)), None);
            chessgame.restore_state(snapshot);
            net.revert(&chessgame, &chessmove);

            node_count += search_data.node_count(); //+ search_data.q_node_count();
            nodes_since_last_check += search_data.node_count(); // + data.q_node_count();

            //if !data.is_aborted() && (eval > best_eval || d > best_d) {
            //    best_eval = eval;
            //    best_move = chess_move.clone();
            //    best_d = d;
            //}

            if eval > best_eval || d > best_d {
                best_eval = eval;
                best_move = chessmove.clone();
                best_d = d;
            }

            if nodes_since_last_check >= LOOP_COUNT_CHECK_LIMIT {
                if now.elapsed() >= hard_time_limit {
                    break;
                }
                nodes_since_last_check = 0;
            }
        }

        if let Err(_) = tx.send((best_move, best_eval, node_count, d)) {
            break;
        }
        //send data
        d += 1;
    }
}

const MILLIS_MARGIN: Duration = Duration::from_millis(10);
const HARD_BASE_COEFF: u32 = 10;
const HARD_INCREMENT_COEFF: u32 = 2;
const SOFT_BASE_COEFF: u32 = 20;
const SOFT_INCREMENT_COEFF: u32 = 3;
