use std::time::{Duration, Instant};

use crate::ChessBoard;

impl ChessBoard {
    pub fn perft_count_timed(&self, depth: usize, is_bulk: bool) -> (u64, Duration) {
        let now = Instant::now();
        let total_count = match is_bulk {
            true => self.perft::<true>(depth),
            false => self.perft::<false>(depth),
        };

        (total_count, now.elapsed())
    }

    pub fn perft_count(&self, depth: usize, is_bulk: bool) -> u64 {
        match is_bulk {
            true => self.perft::<true>(depth),
            false => self.perft::<false>(depth),
        }
    }

    pub fn perft<const IS_BULK: bool>(&self, depth: usize) -> u64 {
        if depth == 0 {
            return 1;
        }

        let moves = self.generate_moves();
        if IS_BULK {
            if depth == 1 {
                return moves.len() as u64;
            }
        }

        let mut total: u64 = 0;
        for chess_move in moves {
            let mut chessboard = *self;
            chessboard.update_state(&chess_move);
            total += chessboard.perft::<IS_BULK>(depth - 1);
        }
        total
    }
}
