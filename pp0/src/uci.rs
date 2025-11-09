use std::{
    io,
    num::NonZero,
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
                    uci_option_info();
                    println!("uciok");
                }
                "position" => {
                    uci_position(&mut chessgame, cmds.collect::<Vec<&str>>().join(" ").as_str(), &mut last_fen);
                    net.initialize(&chessgame);
                }
                "ucinewgame" => {
                    chessgame = ChessGame::start_pos();
                    net.initialize(&chessgame);
                    tt = Arc::new(TT::new());
                    last_fen = Vec::<String>::new();
                }
                "go" => uci_go(&mut chessgame, cmds.collect::<Vec<&str>>().join(" ").as_str(), net, tt.clone()),
                "quit" => return Ok(()),
                "bench" => uci_bench(net),
                //TODO
                _ => {} //???
            }
        }
        buffer.clear();
    }
    //loop {}
    Ok(())
}

const DEFAULT_HASH_MB: usize = TT::size_of() / 1024 / 1024;
pub fn uci_option_info() {
    //
    println!("option name Threads type spin default 1 min 1 max 1");
    println!("option name Hash type spin default {DEFAULT_HASH_MB} min {DEFAULT_HASH_MB} max {DEFAULT_HASH_MB}");
}

const BENCH_DEPTH: NonZero<usize> = NonZero::new(3).unwrap();
pub fn uci_bench(net: &mut Network) {
    let mut total_nodes: usize = 0;
    let mut total_time: Duration = Duration::ZERO;
    for fen in BENCH_FEN {
        let mut chessgame = ChessGame::from_fen(fen);
        let tt: Arc<TT> = Arc::new(TT::new());
        net.initialize(&chessgame);
        let mut data = SearchData::new();
        let (chessmoves, _game_state) = chessgame.try_generate_moves();
        let now = Instant::now();
        _ = data.find_move(&mut chessgame, net, tt, &SearchLimit::Depth(BENCH_DEPTH), &chessmoves);
        total_time += now.elapsed();
        total_nodes += data.node_count()
    }
    let nps = (total_nodes as f64 / total_time.as_secs_f64()) as usize;
    println!("{} nodes {} nps", total_nodes, nps);
    //
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
        let mut loop_counter: usize = 0;
        while duration < hard_time_limit && d <= max_depth {
            if loop_counter >= 2048 {
                duration = now.elapsed();
            }
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
            loop_counter += 1;
        }

        println!("bestmove {}", best_move.print_move());
    });

    let mut nodes_since_last_check: usize = 0;
    //search_data.find_move(chess_game, net, tt.clone(), &time_limit, moves);

    'search: while now.elapsed() < soft_time_limit && d <= max_depth {
        let mut search_data: SearchData = SearchData::new();
        let mut best_eval: i16 = i16::MIN + 1;
        net.update(&chessgame, &best_move);
        let snapshot: chessbb::ChessBoardSnapshot = chessgame.explore_state(&best_move);
        let eval: i16 =
            -search_data.negamax::<true, false>(chessgame, best_eval, i16::MAX - 1, d as usize - 1, net, tt.clone(), Some((now, hard_time_limit)), None);
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
                -search_data.negamax::<true, false>(chessgame, best_eval, i16::MAX - 1, d as usize - 1, net, tt.clone(), Some((now, hard_time_limit)), None);
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

// fens from Stormphrax
static BENCH_FEN: [&str; 50] = [
    "r3k2r/2pb1ppp/2pp1q2/p7/1nP1B3/1P2P3/P2N1PPP/R2QK2R w KQkq - 0 14",
    "4rrk1/2p1b1p1/p1p3q1/4p3/2P2n1p/1P1NR2P/PB3PP1/3R1QK1 b - - 2 24",
    "r3qbrk/6p1/2b2pPp/p3pP1Q/PpPpP2P/3P1B2/2PB3K/R5R1 w - - 16 42",
    "6k1/1R3p2/6p1/2Bp3p/3P2q1/P7/1P2rQ1K/5R2 b - - 4 44",
    "8/8/1p2k1p1/3p3p/1p1P1P1P/1P2PK2/8/8 w - - 3 54",
    "7r/2p3k1/1p1p1qp1/1P1Bp3/p1P2r1P/P7/4R3/Q4RK1 w - - 0 36",
    "r1bq1rk1/pp2b1pp/n1pp1n2/3P1p2/2P1p3/2N1P2N/PP2BPPP/R1BQ1RK1 b - - 2 10",
    "3r3k/2r4p/1p1b3q/p4P2/P2Pp3/1B2P3/3BQ1RP/6K1 w - - 3 87",
    "2r4r/1p4k1/1Pnp4/3Qb1pq/8/4BpPp/5P2/2RR1BK1 w - - 0 42",
    "4q1bk/6b1/7p/p1p4p/PNPpP2P/KN4P1/3Q4/4R3 b - - 0 37",
    "2q3r1/1r2pk2/pp3pp1/2pP3p/P1Pb1BbP/1P4Q1/R3NPP1/4R1K1 w - - 2 34",
    "1r2r2k/1b4q1/pp5p/2pPp1p1/P3Pn2/1P1B1Q1P/2R3P1/4BR1K b - - 1 37",
    "r3kbbr/pp1n1p1P/3ppnp1/q5N1/1P1pP3/P1N1B3/2P1QP2/R3KB1R b KQkq - 0 17",
    "8/6pk/2b1Rp2/3r4/1R1B2PP/P5K1/8/2r5 b - - 16 42",
    "1r4k1/4ppb1/2n1b1qp/pB4p1/1n1BP1P1/7P/2PNQPK1/3RN3 w - - 8 29",
    "8/p2B4/PkP5/4p1pK/4Pb1p/5P2/8/8 w - - 29 68",
    "3r4/ppq1ppkp/4bnp1/2pN4/2P1P3/1P4P1/PQ3PBP/R4K2 b - - 2 20",
    "5rr1/4n2k/4q2P/P1P2n2/3B1p2/4pP2/2N1P3/1RR1K2Q w - - 1 49",
    "1r5k/2pq2p1/3p3p/p1pP4/4QP2/PP1R3P/6PK/8 w - - 1 51",
    "q5k1/5ppp/1r3bn1/1B6/P1N2P2/BQ2P1P1/5K1P/8 b - - 2 34",
    "r1b2k1r/5n2/p4q2/1ppn1Pp1/3pp1p1/NP2P3/P1PPBK2/1RQN2R1 w - - 0 22",
    "r1bqk2r/pppp1ppp/5n2/4b3/4P3/P1N5/1PP2PPP/R1BQKB1R w KQkq - 0 5",
    "r1bqr1k1/pp1p1ppp/2p5/8/3N1Q2/P2BB3/1PP2PPP/R3K2n b Q - 1 12",
    "r1bq2k1/p4r1p/1pp2pp1/3p4/1P1B3Q/P2B1N2/2P3PP/4R1K1 b - - 2 19",
    "r4qk1/6r1/1p4p1/2ppBbN1/1p5Q/P7/2P3PP/5RK1 w - - 2 25",
    "r7/6k1/1p6/2pp1p2/7Q/8/p1P2K1P/8 w - - 0 32",
    "r3k2r/ppp1pp1p/2nqb1pn/3p4/4P3/2PP4/PP1NBPPP/R2QK1NR w KQkq - 1 5",
    "3r1rk1/1pp1pn1p/p1n1q1p1/3p4/Q3P3/2P5/PP1NBPPP/4RRK1 w - - 0 12",
    "5rk1/1pp1pn1p/p3Brp1/8/1n6/5N2/PP3PPP/2R2RK1 w - - 2 20",
    "8/1p2pk1p/p1p1r1p1/3n4/8/5R2/PP3PPP/4R1K1 b - - 3 27",
    "8/4pk2/1p1r2p1/p1p4p/Pn5P/3R4/1P3PP1/4RK2 w - - 1 33",
    "8/5k2/1pnrp1p1/p1p4p/P6P/4R1PK/1P3P2/4R3 b - - 1 38",
    "8/8/1p1kp1p1/p1pr1n1p/P6P/1R4P1/1P3PK1/1R6 b - - 15 45",
    "8/8/1p1k2p1/p1prp2p/P2n3P/6P1/1P1R1PK1/4R3 b - - 5 49",
    "8/8/1p4p1/p1p2k1p/P2npP1P/4K1P1/1P6/3R4 w - - 6 54",
    "8/8/1p4p1/p1p2k1p/P2n1P1P/4K1P1/1P6/6R1 b - - 6 59",
    "8/5k2/1p4p1/p1pK3p/P2n1P1P/6P1/1P6/4R3 b - - 14 63",
    "8/1R6/1p1K1kp1/p6p/P1p2P1P/6P1/1Pn5/8 w - - 0 67",
    "1rb1rn1k/p3q1bp/2p3p1/2p1p3/2P1P2N/PP1RQNP1/1B3P2/4R1K1 b - - 4 23",
    "4rrk1/pp1n1pp1/q5p1/P1pP4/2n3P1/7P/1P3PB1/R1BQ1RK1 w - - 3 22",
    "r2qr1k1/pb1nbppp/1pn1p3/2ppP3/3P4/2PB1NN1/PP3PPP/R1BQR1K1 w - - 4 12",
    "2r2k2/8/4P1R1/1p6/8/P4K1N/7b/2B5 b - - 0 55",
    "6k1/5pp1/8/2bKP2P/2P5/p4PNb/B7/8 b - - 1 44",
    "2rqr1k1/1p3p1p/p2p2p1/P1nPb3/2B1P3/5P2/1PQ2NPP/R1R4K w - - 3 25",
    "r1b2rk1/p1q1ppbp/6p1/2Q5/8/4BP2/PPP3PP/2KR1B1R b - - 2 14",
    "6r1/5k2/p1b1r2p/1pB1p1p1/1Pp3PP/2P1R1K1/2P2P2/3R4 w - - 1 36",
    "rnbqkb1r/pppppppp/5n2/8/2PP4/8/PP2PPPP/RNBQKBNR b KQkq - 0 2",
    "2rr2k1/1p4bp/p1q1p1p1/4Pp1n/2PB4/1PN3P1/P3Q2P/2RR2K1 w - f6 0 20",
    "3br1k1/p1pn3p/1p3n2/5pNq/2P1p3/1PN3PP/P2Q1PB1/4R1K1 w - - 0 23",
    "2r2b2/5p2/5k2/p1r1pP2/P2pB3/1P3P2/K1P3R1/7R w - - 23 93",
];
