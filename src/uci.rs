use std::io;

use crate::chessnet::{ChessGame, ChessNet};

//const DEBUG: bool = false;

impl ChessNet {
    pub fn uci_loop_start(&mut self) -> io::Result<()> {
        let mut chessgame = ChessGame::start_pos();
        let mut reader = io::BufReader::new(io::stdin());
        let mut buffer = String::with_capacity(1 << 12);
        while let Ok(count) = io::BufRead::read_line(&mut reader, &mut buffer) {
            //if DEBUG {
            //    print!("buffer:{}", buffer);
            //}

            if count == 0 {
                return Ok(());
            }

            let mut cmds = buffer.split_whitespace();
            if let Some(cmd) = cmds.next() {
                match cmd {
                    "isready" => println!("readyok"),
                    "uci" => {
                        println!("id name pp0");
                        println!("id author Fangs");
                        println!("uciok");
                    }
                    "position" => uci_position(&mut chessgame, cmds.collect::<Vec<&str>>().join(" ").as_str()),
                    "ucinewgame" => _ = uci_go(&mut chessgame, "startpos", self),
                    "go" => {
                        println!("{}", uci_go(&mut chessgame, cmds.collect::<Vec<&str>>().join(" ").as_str(), self))
                    }
                    "quit" => return Ok(()),
                    //TODO
                    _ => {} //???
                }
            }
            buffer.clear();
        }

        return Ok(());
    }
}

fn uci_position(chessgame: &mut ChessGame, cmd_str: &str) {
    let mut cmds = cmd_str.split(' ');
    let mut is_parsing_moves = false;
    'commands: while let Some(cmd) = cmds.next() {
        if !is_parsing_moves {
            match cmd {
                "startpos" => *chessgame = ChessGame::start_pos(),
                "FEN" | "fen" => {
                    if let Some(fen) = cmds.next() {
                        *chessgame = ChessGame::from_fen(fen);
                    } else {
                        *chessgame = ChessGame::start_pos();
                    }
                }
                "MOVES" | "moves" => {
                    is_parsing_moves = true;
                }
                //TODO
                _ => panic!("unknown command"),
            }
        } else {
            for chess_move in chessgame.try_generate_moves().0 {
                if chess_move.print_move() == cmd {
                    //todo: maybe parse into a source/target and do int compare
                    chessgame.update_state(chess_move);
                    continue 'commands;
                }
            }
            panic!("invalid move")
        }
    }
}

pub fn uci_go(chessgame: &mut ChessGame, cmd_str: &str, net: &mut ChessNet) -> String {
    let mut depth: usize = 3;

    let mut cmds = cmd_str.split(' ');

    while let Some(cmd) = cmds.next() {
        // fix depth search
        if cmd == "depth" {
            //todo: fix this lazy shit
            depth = cmds.next().expect("Invalid input").parse::<usize>().expect("Invalid input");
        }
        // other cases placeholder
        else {
            depth = 3;
        }
        //other cases?
    }
    //search_position(depth)
    return format!("bestmove {}", net.negamax_cold(chessgame, depth).print_move());
}
