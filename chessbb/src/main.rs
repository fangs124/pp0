//use chessbb::chessmove::*;
use chessbb::*;
use core::error;
//use chessbb::chessmove::ChessMove;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;

extern crate chessbb;
fn main() {
    let is_bulk = false;
    perft_suite(None, is_bulk);
    //let fen = "k7/8/8/7p/6P1/8/8/K7 w - - 0 1";
    //let raw_moves: Vec<u16> = vec![2526];
    //perft_test(fen, raw_moves, is_bulk);
}
const START_DEPTH: usize = 2;
const MAX_DEPTH: usize = 2;
const PANIC_ON_ERROR: bool = true;
fn perft_test(fen: &str, raw_moves: Vec<u16>, is_bulk: bool) {
    println!("\n============== history ===============");
    println!("fen: {fen}");
    let mut chessboard = ChessBoard::from_fen(fen);
    for data in raw_moves {
        println!("{}", chessboard.print_board());
        chessboard.update_state(&ChessMove::from_raw(data));
    }

    println!("\n============== position ==============");
    println!("{}", chessboard.print_board());
    println!("{}", chessboard.print_board_debug());
    println!("======================================\n");
    let mut depth = START_DEPTH;
    let mut moves = chessboard.generate_moves();
    moves.sort_by(LexiOrd::lexi_cmp);
    while depth <= MAX_DEPTH {
        let mut result_str_vec = Vec::<String>::new();
        let mut total_count: u64 = 0;
        for chess_move in moves.clone() {
            let mut s = chess_move.print_move();
            let mut new_chessboard = chessboard.clone();
            new_chessboard.update_state(&chess_move);
            let branch_total: u64 = new_chessboard.perft_count(depth - 1, is_bulk);
            total_count += branch_total;
            s.push_str(format!(" - {:<6}", branch_total).as_str());
            s.push_str(format!(" - data: {}", chess_move.data()).as_str());
            result_str_vec.push(s);
        }

        println!("depth: {depth}, total_count: {total_count}");
        for result_str in result_str_vec {
            println!("{result_str}");
        }
        println!("");
        depth += 1;
    }
}

fn perft_suite(skip_to: Option<usize>, is_bulk: bool) {
    let mut node_count: u64 = 0;
    let path = Path::new("standard.epd");
    let display = path.display();

    let mut file = match File::open(&path) {
        Err(why) => panic!("couldn't open {display}: {why}"),
        Ok(file) => file,
    };

    let mut s = String::new();
    match file.read_to_string(&mut s) {
        Err(why) => panic!("couldn't read {}: {}", display, why),
        Ok(_) => print!("{} contains:\n{}", display, s),
    }
    let lines: Vec<&str> = s.split('\n').collect();
    let mut num: usize = 0;
    let mut elapsed_total: Duration = Duration::new(0, 0);
    let mut has_error = false;
    for line in lines {
        let mut sections = line.split(';');

        let start_fen = sections.next().unwrap();
        num += 1;
        if skip_to.is_some() {
            if num != skip_to.unwrap() {
                continue;
            }
        }
        let chessboard = ChessBoard::from_fen(start_fen);
        println!("\n========= position number {:3<} =========", num);
        println!("fen: {start_fen}");
        println!("=======================================\n");
        println!("{}", chessboard.print_board());
        //println!("{}", chessboard.print_board_debug());

        for section in sections {
            let section_vec: Vec<_> = section.split_ascii_whitespace().collect();
            let depth: usize = section_vec[0].chars().filter(|x| x.is_ascii_digit()).collect::<String>().parse().unwrap();
            let result_count: u64 = section_vec[1].parse().unwrap();

            let (total_count, elapsed) = chessboard.perft_count_timed(depth, is_bulk);
            elapsed_total += elapsed;

            let result_str = match result_count == total_count {
                true => "Ok!",
                false => "Error!",
            };

            println!("depth: {depth}, result_count: {result_count}, total_count: {total_count}, {result_str}");
            if result_str == "Error!" {
                has_error = true;
                println!("");
                if PANIC_ON_ERROR {
                    panic!("fen:{}, depth: {}", section, depth);
                }
            }
            node_count += total_count;
        }
    }

    if has_error {
    } else {
        println!("done... no error!");
    }
    println!("total positions: {node_count}, time: {}ms", elapsed_total.as_millis());
    println!("speed: {:.2}Mnps", ((node_count as f64) / (1000000.0)) / elapsed_total.as_secs_f64());
}
