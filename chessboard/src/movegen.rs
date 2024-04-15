use super::chessmove::*;
use super::*;

pub(super) const fn update_state(chessboard: &ChessBoard, chess_move: ChessMove) -> ChessBoard {
    let mut chessboard = chessboard.const_clone();
    let mut enpassant_bb: BitBoard = BitBoard::ZERO;
    let source: usize = chess_move.source();
    let target: usize = chess_move.target();
    let source_data = match chessboard.mailbox[source] {
        Some(x) => x,
        None => panic!("update_state error: source mailbox is None!"),
    };
    let source_index = cp_index(source_data);
    assert!(chessboard.hash_vec.len() > 0);
    let mut current_hash = chessboard.get_hash().val;

    // handle special cases
    match chessboard.mailbox[source] {
        opt_cpt!(_) => panic!("update_state error: source mailbox is None!"),

        /* special case: castling rights */
        opt_cpt!(K) => {
            chessboard.castle_bools[0] = false;
            chessboard.castle_bools[1] = false;
        }
        opt_cpt!(R) => {
            if source == 0 {
                chessboard.castle_bools[0] = false;
            } else if source == 7 {
                chessboard.castle_bools[1] = false
            }
        }
        opt_cpt!(k) => {
            chessboard.castle_bools[2] = false;
            chessboard.castle_bools[3] = false;
        }
        opt_cpt!(r) => {
            if source == 56 {
                chessboard.castle_bools[2] = false;
            } else if source == 63 {
                chessboard.castle_bools[3] = false
            }
        }

        /* special case: pawn 2-squares move, en passant rules */
        opt_cpt!(P) => {
            //reset 50-move rule
            chessboard.moverule_counter = u16::MAX;
            // check if move is 2-square
            if source + 16 == target {
                if target + 1 < 64 {
                    // check pawn lands next to enemy pawn
                    match chessboard.mailbox[target + 1] {
                        opt_cpt!(p) => {
                            //check if pawn is not pinned
                            if !chessboard.is_piece_pinned(target + 1) {
                                enpassant_bb = enpassant_bb.bit_and(&BitBoard::nth(target - 8));
                            }
                        }
                        _ => {}
                    }
                }

                if 0 + 1 <= target {
                    // unsigned hack: 0 <= target - 1
                    // check pawn lands next to enemy pawn
                    match chessboard.mailbox[target - 1] {
                        opt_cpt!(p) => {
                            //check if pawn is not pinned
                            if !chessboard.is_piece_pinned(target - 1) {
                                enpassant_bb = enpassant_bb.bit_and(&BitBoard::nth(target - 8));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        opt_cpt!(p) => {
            //reset 50-move rule
            chessboard.moverule_counter = u16::MAX;
            if source == target + 16 {
                // unsinged hack: source - 16 == target
                if target + 1 < 64 {
                    // check pawn lands next to enemy pawn
                    match chessboard.mailbox[target + 1] {
                        opt_cpt!(p) => {
                            //check if pawn is not pinned
                            if !chessboard.is_piece_pinned(target + 1) {
                                enpassant_bb = enpassant_bb.bit_and(&BitBoard::nth(target + 8));
                            }
                        }
                        _ => {}
                    }
                }

                if 0 + 1 <= target {
                    // unsigned hack: 0 <= target - 1
                    // check pawn lands next to enemy pawn
                    match chessboard.mailbox[target - 1] {
                        opt_cpt!(p) => {
                            //check if pawn is not pinned
                            if !chessboard.is_piece_pinned(target - 1) {
                                enpassant_bb = enpassant_bb.bit_and(&BitBoard::nth(target + 8));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        _ => {}
    }

    // update piece_bbs and mailbox
    match chess_move.get_move_type() {
        MoveType::Normal => {
            // if source is a pawn and move is two-squares, encode enpassant data
            match source_data {
                cpt!(P) => {
                    if source + 16 == target {
                        enpassant_bb = enpassant_bb.bit_or(&BitBoard::nth(target - 8));
                    }
                }

                cpt!(p) => {
                    if source == target + 16 {
                        //source - 16 == target
                        enpassant_bb = enpassant_bb.bit_or(&BitBoard::nth(target + 8));
                    }
                }

                _ => {}
            }

            // update source bitboard
            chessboard.piece_bbs[source_index] = chessboard.piece_bbs[source_index].bit_and(&BitBoard::nth(source).bit_not());
            chessboard.piece_bbs[source_index] = chessboard.piece_bbs[source_index].bit_or(&BitBoard::nth(target));

            //update hash
            current_hash ^= ZorbistHash::get_piece_hash(source, source_data);
            current_hash ^= ZorbistHash::get_piece_hash(target, source_data);

            // if target is occupied, deal with piece capture
            if let Some(target_data) = chessboard.mailbox[target] {
                let target_index = cp_index(target_data);
                //reset 50-move rule
                chessboard.moverule_counter = u16::MAX;
                chessboard.piece_bbs[target_index] = chessboard.piece_bbs[target_index].bit_and(&BitBoard::nth(target).bit_not());

                //update hash
                current_hash ^= ZorbistHash::get_piece_hash(target, target_data);

                match target_data {
                    cpt!(R) => {
                        if target == 0 {
                            chessboard.castle_bools[0] = false;
                        } else if target == 7 {
                            chessboard.castle_bools[1] = false
                        }
                    }

                    cpt!(r) => {
                        if target == 56 {
                            chessboard.castle_bools[2] = false;
                        } else if target == 63 {
                            chessboard.castle_bools[3] = false
                        }
                    }
                    _ => {}
                }
            }

            // update mailbox
            chessboard.mailbox[source] = None;
            chessboard.mailbox[target] = Some(source_data);
        }

        MoveType::Castle => {
            // update source bitboard
            chessboard.piece_bbs[source_index] = chessboard.piece_bbs[source_index].bit_and(&BitBoard::nth(source).bit_not());
            chessboard.piece_bbs[source_index] = chessboard.piece_bbs[source_index].bit_or(&BitBoard::nth(target));

            //update hash
            current_hash ^= ZorbistHash::get_piece_hash(source, source_data);
            current_hash ^= ZorbistHash::get_piece_hash(target, source_data);

            // update mailbox
            chessboard.mailbox[source] = None;
            chessboard.mailbox[target] = Some(source_data);

            // deal with rook movement
            match (chessboard.side_to_move, target) {
                // white king-side castle
                (Side::White, 1) => {
                    // check if rook is present
                    assert!(chessboard.piece_bbs[04].nth_is_not_zero(0));
                    //assert!(chessboard.piece_bbs[04].data & 1u64 << 00 != 0, "board:{}", chessboard);
                    chessboard.piece_bbs[04] = chessboard.piece_bbs[04].bit_and(&BitBoard::nth(0).bit_not());
                    chessboard.piece_bbs[04] = chessboard.piece_bbs[04].bit_or(&BitBoard::nth(2));
                    chessboard.mailbox[00] = None;
                    chessboard.mailbox[02] = opt_cpt!(R);

                    //update hash
                    current_hash ^= ZorbistHash::get_piece_hash(00, cpt!(R));
                    current_hash ^= ZorbistHash::get_piece_hash(02, cpt!(R));
                }

                // white queen-side castle
                (Side::White, 5) => {
                    // check if rook is present
                    assert!(chessboard.piece_bbs[04].nth_is_not_zero(7));
                    chessboard.piece_bbs[04] = chessboard.piece_bbs[04].bit_and(&BitBoard::nth(7).bit_not());
                    chessboard.piece_bbs[04] = chessboard.piece_bbs[04].bit_or(&BitBoard::nth(4));
                    chessboard.mailbox[07] = None;
                    chessboard.mailbox[04] = opt_cpt!(R);

                    //update hash
                    current_hash ^= ZorbistHash::get_piece_hash(07, cpt!(R));
                    current_hash ^= ZorbistHash::get_piece_hash(02, cpt!(R));
                }

                // black king-side castle
                (Side::Black, 57) => {
                    // check if rook is present
                    assert!(chessboard.piece_bbs[10].nth_is_not_zero(56));
                    chessboard.piece_bbs[10] = chessboard.piece_bbs[10].bit_and(&BitBoard::nth(56).bit_not());
                    chessboard.piece_bbs[10] = chessboard.piece_bbs[10].bit_or(&BitBoard::nth(58));
                    chessboard.mailbox[56] = None;
                    chessboard.mailbox[58] = opt_cpt!(r);

                    //update hash
                    current_hash ^= ZorbistHash::get_piece_hash(56, cpt!(r));
                    current_hash ^= ZorbistHash::get_piece_hash(58, cpt!(r));
                }

                (Side::Black, 61) => {
                    // check if rook is present
                    assert!(chessboard.piece_bbs[10].nth_is_not_zero(63));
                    chessboard.piece_bbs[10] = chessboard.piece_bbs[10].bit_and(&BitBoard::nth(63).bit_not());
                    chessboard.piece_bbs[10] = chessboard.piece_bbs[10].bit_or(&BitBoard::nth(60));
                    chessboard.mailbox[63] = None;
                    chessboard.mailbox[60] = opt_cpt!(r);

                    //update hash
                    current_hash ^= ZorbistHash::get_piece_hash(63, cpt!(r));
                    current_hash ^= ZorbistHash::get_piece_hash(60, cpt!(r));
                }

                _ => panic!("update_state error: invalid castling target!"),
            }
        }

        MoveType::EnPassant => {
            // note: target is where the capturing pawn will end up,
            //       square is where the pawn to be captured is

            // update source bitboard
            chessboard.piece_bbs[source_index] = chessboard.piece_bbs[source_index].bit_and(&BitBoard::nth(source).bit_not());
            chessboard.piece_bbs[source_index] = chessboard.piece_bbs[source_index].bit_or(&BitBoard::nth(target));

            //update hash
            current_hash ^= ZorbistHash::get_piece_hash(source, source_data);
            current_hash ^= ZorbistHash::get_piece_hash(target, source_data);

            let index = match chessboard.side_to_move {
                Side::White => 11usize,
                Side::Black => 05usize,
            };

            let square = match chessboard.side_to_move {
                Side::White => target - 8,
                Side::Black => target + 8,
            };

            // check presence of pawn to be captured
            assert!(chessboard.piece_bbs[index].nth_is_not_zero(square));

            // assert!(chessboard.mailbox[square] == Some(relevant_piece));
            if let Some(piece) = chessboard.mailbox[square] {
                //note: assert hack
                match chessboard.side_to_move {
                    Side::White => match piece {
                        cpt!(p) => {}
                        _ => panic!("update_state error: square mailbox is not pawn, en_passant case!"),
                    },
                    Side::Black => match piece {
                        cpt!(P) => {}
                        _ => panic!("update_state error: square mailbox is not pawn, en_passant case!"),
                    },
                }
            } else {
                panic!("update_state error: en passant square mailbox is None!")
            }

            // deal with piece capture
            let square_data = match chessboard.mailbox[square] {
                Some(x) => x,
                None => panic!("update_state error: en passant square mailbox is None!"),
            };
            let target_index = cp_index(square_data);
            chessboard.piece_bbs[target_index] = chessboard.piece_bbs[target_index].bit_and(&BitBoard::nth(square).bit_not());

            //update hash
            current_hash ^= ZorbistHash::get_piece_hash(square, square_data);

            // update mailbox
            chessboard.mailbox[source] = None;
            chessboard.mailbox[target] = Some(source_data);
            chessboard.mailbox[square] = None;
        }

        MoveType::Promotion => {
            let promotion_piece = match chess_move.get_piece_data() {
                Some(x) => x,
                None => panic!("update_state error: chess_move is a promotion with None piece data!"),
            };

            let new_piece = (chessboard.side_to_move, promotion_piece);
            let target_index = cp_index(new_piece);

            // update bitboards
            chessboard.piece_bbs[source_index] = chessboard.piece_bbs[source_index].bit_and(&BitBoard::nth(source));
            chessboard.piece_bbs[target_index] = chessboard.piece_bbs[target_index].bit_or(&BitBoard::nth(target));

            //update hash
            current_hash ^= ZorbistHash::get_piece_hash(source, source_data);
            current_hash ^= ZorbistHash::get_piece_hash(target, new_piece);

            // if target is occupied, deal with piece capture
            if let Some(data_target) = chessboard.mailbox[target] {
                let target_index = cp_index(new_piece);
                chessboard.piece_bbs[target_index] = chessboard.piece_bbs[target_index].bit_and(&BitBoard::nth(target).bit_not());

                //update hash
                current_hash ^= ZorbistHash::get_piece_hash(target, data_target);

                match data_target {
                    cpt!(R) => {
                        if target == 0 {
                            chessboard.castle_bools[0] = false;
                        } else if target == 7 {
                            chessboard.castle_bools[1] = false
                        }
                    }

                    cpt!(r) => {
                        if target == 56 {
                            chessboard.castle_bools[2] = false;
                        } else if target == 63 {
                            chessboard.castle_bools[3] = false
                        }
                    }
                    _ => {}
                }
            }

            // update mailbox
            chessboard.mailbox[source] = None;
            chessboard.mailbox[target] = Some(new_piece);
        }
    }

    chessboard.enpassant_bb = enpassant_bb;
    match chessboard.side_to_move {
        Side::Black => chessboard.full_move_counter += 1,
        _____ => {}
    }
    chessboard.side_to_move = chessboard.side_to_move.update();
    chessboard.half_move_clock += 1;

    // ['K','Q','N','B','R','P','k','q','n','b','r','p'];
    //check if move results in opponent's king to be in check
    match chessboard.king_is_in_check(chessboard.side_to_move) {
        true => {
            match chessboard.side_to_move {
                Side::White => {
                    let blockers = chessboard.blockers();
                    if let Some(king_pos) = chessboard.piece_bbs[0].lsb_index() {
                        let mut check_bitboard = BitBoard::ZERO;
                        //q
                        let bitboard = chessboard.piece_bbs[07].bit_and(&get_queen_attack(king_pos, blockers));
                        check_bitboard = check_bitboard.bit_or(&bitboard);
                        //n
                        let bitboard = chessboard.piece_bbs[08].bit_and(&get_knight_attack(king_pos));
                        check_bitboard = check_bitboard.bit_or(&bitboard);
                        //b
                        let bitboard = chessboard.piece_bbs[09].bit_and(&get_bishop_attack(king_pos, blockers));
                        check_bitboard = check_bitboard.bit_or(&bitboard);
                        //r
                        let bitboard = chessboard.piece_bbs[10].bit_and(&get_rook_attack(king_pos, blockers));
                        check_bitboard = check_bitboard.bit_or(&bitboard);
                        //p
                        let bitboard = chessboard.piece_bbs[11].bit_and(&get_b_pawn_attack(king_pos));
                        check_bitboard = check_bitboard.bit_or(&bitboard);
                        chessboard.check_bb = check_bitboard;
                    } else {
                        panic!("update_state error: white king bitboard is empty!");
                    }
                }

                Side::Black => {
                    let blockers = chessboard.blockers();
                    if let Some(king_pos) = chessboard.piece_bbs[6].lsb_index() {
                        let mut check_bitboard = BitBoard::ZERO;
                        //Q
                        let bitboard = chessboard.piece_bbs[01].bit_and(&get_queen_attack(king_pos, blockers));
                        check_bitboard = check_bitboard.bit_or(&bitboard);
                        //N
                        let bitboard = chessboard.piece_bbs[02].bit_and(&get_knight_attack(king_pos));
                        check_bitboard = check_bitboard.bit_or(&bitboard);
                        //B
                        let bitboard = chessboard.piece_bbs[03].bit_and(&get_bishop_attack(king_pos, blockers));
                        check_bitboard = check_bitboard.bit_or(&bitboard);
                        //R
                        let bitboard = chessboard.piece_bbs[04].bit_and(&get_rook_attack(king_pos, blockers));
                        check_bitboard = check_bitboard.bit_or(&bitboard);
                        //P
                        let bitboard = chessboard.piece_bbs[05].bit_and(&get_b_pawn_attack(king_pos));
                        check_bitboard = check_bitboard.bit_or(&bitboard);
                        chessboard.check_bb = check_bitboard;
                    } else {
                        panic!("update_state error: black king bitboard is empty!");
                    }
                }
            }
        }
        false => {
            chessboard.check_bb = BitBoard::ZERO;
        }
    }

    let mut enpassant_bb = chessboard.enpassant_bb;
    while enpassant_bb.is_not_zero() {
        let square = match enpassant_bb.lsb_index() {
            Some(x) => x,
            None => unreachable!(),
        };
        current_hash ^= FILE_HASH[COLS[square]];
        enpassant_bb = enpassant_bb.pop_bit(square);
    }

    // lazy way to handle this
    let mut i: usize = 0;
    //castling hash
    while i < 4 {
        if chessboard.castle_bools[i] {
            current_hash ^= CASTLE_HASH[i];
        }
        i += 1;
    }

    //en passant hash
    let mut enpassant_bb = chessboard.enpassant_bb;
    while enpassant_bb.is_not_zero() {
        let square = match enpassant_bb.lsb_index() {
            Some(x) => x,
            None => unreachable!(),
        };
        current_hash ^= FILE_HASH[COLS[square]];
        enpassant_bb = enpassant_bb.pop_bit(square);
    }

    //side to move hash
    current_hash ^= SIDE_HASH[0];
    chessboard.hash_vec = chessboard.hash_vec.append_one(ZorbistHash { val: current_hash });

    //move principal variation forward
    //if self.pv.len() > 0 {
    //    chessboard.pv.count = self.pv.count - 1;
    //    chessboard.pv.data = [None; 256];
    //    let mut i: usize = 0;
    //    while i + 1 < self.pv.count {
    //        chessboard.pv.data[i] = self.pv.data[i + 1];
    //        i += 1;
    //    }
    //}

    //update game state value
    //chessboard.history = chessboard.get_history();

    if chessboard.moverule_counter == u16::MAX {
        chessboard.moverule_counter = 0;
    } else {
        chessboard.moverule_counter += 1;
    }
    chessboard.moves = Some((chessboard.generate_moves(), chessboard.get_hash()));
    chessboard.state = chessboard.get_state();
    return chessboard;
}

pub(super) const fn generate_moves(chessboard: &ChessBoard) -> MoveVec {
    //assert!(
    //    self.piece_bbs[0].data.count_ones() == 1 && self.piece_bbs[6].data.count_ones() == 1
    //);
    let mut arr = MoveVec::new();
    if chessboard.hash_count(chessboard.get_hash()) == 3 {
        return arr;
    };
    let blockers = chessboard.blockers();
    let w_blockers = chessboard.white_blockers();
    let b_blockers = chessboard.black_blockers();
    let side = chessboard.side_to_move;
    let king_pos = match side {
        Side::White => match chessboard.piece_bbs[0].lsb_index() {
            Some(x) => x,
            None => unreachable!(),
        },
        Side::Black => match chessboard.piece_bbs[6].lsb_index() {
            Some(x) => x,
            None => unreachable!(),
        },
    };

    let enemies = match side {
        Side::White => b_blockers,
        Side::Black => w_blockers,
    };

    let friends = match side {
        Side::White => w_blockers,
        Side::Black => b_blockers,
    };

    // consider if king is in check
    let mut check_mask: BitBoard = chessboard.check_bb;
    let checkers_count = chessboard.check_bb.count_ones();
    if chessboard.check_bb.is_not_zero() {
        let mut checkers = chessboard.check_bb;
        let index: usize = match side {
            Side::White => 0,
            Side::Black => 6,
        };

        let k: usize = match chessboard.piece_bbs[index].lsb_index() {
            Some(x) => x,
            None => panic!("generate_moves error: king not found!"),
        };

        while checkers.is_not_zero() {
            let i: usize = match checkers.lsb_index() {
                Some(x) => x,
                None => unreachable!(),
            };

            if let Some(piece) = chessboard.mailbox[i] {
                match piece {
                    cpt!(K) | cpt!(k) => {
                        panic!("generate_moves error: king is in check by another king!")
                    }
                    cpt!(N) | cpt!(n) => {
                        check_mask = check_mask.bit_or(&get_knight_attack(i).bit_and(&get_knight_attack(k)));
                    }
                    _ => {
                        check_mask = check_mask.bit_or(&RAYS[i][k]);
                    } /*
                      cpt!(Q) | cpt!(q) => {
                          check_mask.data |=  RAYS[i][k].data;
                      }
                      cpt!(B) | cpt!(b) => {
                          check_mask.data |= RAYS[i][k].data;
                      }
                      cpt!(R) | cpt!(r) => {
                          check_mask.data |= RAYS[i][k].data;
                      }
                      cpt!(P) | cpt!(p) => {
                          check_mask.data |= RAYS[i][k].data;
                      }
                      */
                }
            }
            checkers = checkers.pop_bit(i)
        }
        //debug
        //println!("check_mask:");
        //println!("{}", check_mask);
    }

    let mut i: usize = match side {
        Side::White => 0,
        Side::Black => 6,
    };

    let limit = i + 6;
    while i < limit {
        let mut sources = chessboard.piece_bbs[i];
        while sources.is_not_zero() {
            let source: usize = match sources.lsb_index() {
                Some(x) => x,
                None => unreachable!(),
            };

            // pin information
            let mut pinners = BB::ZERO;
            let mut pin_mask = BB::ZERO;
            let is_pinned = match i {
                0 | 6 => false,
                _____ => chessboard.is_piece_pinned(source),
            };
            if is_pinned {
                let piece_bbs = chessboard.piece_bbs;
                let (q_index, b_index, r_index) = match side {
                    Side::White => (07, 09, 10),
                    Side::Black => (01, 03, 04),
                };
                let diagonals = (piece_bbs[q_index].bit_or(&piece_bbs[b_index])).bit_and(&BitBoard::nth(source).bit_not());
                let laterals = (piece_bbs[q_index].bit_or(&piece_bbs[r_index])).bit_and(&BitBoard::nth(source).bit_not());
                let bb1 = get_bishop_attack(king_pos, diagonals).bit_and(&diagonals);
                let bb2 = get_rook_attack(king_pos, laterals).bit_and(&laterals);
                let mut potential_pinners: BitBoard = enemies.bit_and(&bb1.bit_or(&bb2));
                while potential_pinners.is_not_zero() {
                    let potential_pinner = match potential_pinners.lsb_index() {
                        Some(x) => x,
                        None => unreachable!(),
                    };
                    // check if piece is between king and potential_pinner
                    if RAYS[king_pos][potential_pinner].nth_is_not_zero(source) {
                        pinners = pinners.bit_or(&BitBoard::nth(potential_pinner));
                        pin_mask = pin_mask.bit_or(&RAYS[king_pos][potential_pinner].bit_or(&BB::nth(potential_pinner)));
                    }
                    potential_pinners = potential_pinners.pop_bit(potential_pinner);
                }
            }

            match i {
                /* king */
                00 | 06 => {
                    /* castling */
                    if chessboard.check_bb.is_zero() {
                        // can not castle whilst in check
                        let (k_mask, k_index) = match side {
                            Side::White => (W_KING_SIDE_CASTLE_MASK, 0),
                            Side::Black => (B_KING_SIDE_CASTLE_MASK, 2),
                        };
                        // king-side
                        if chessboard.castle_bools[k_index] && blockers.bit_and(&k_mask).is_zero() {
                            //check if squares are under attack
                            let mut squares = k_mask;
                            let mut can_castle = true;
                            while squares.is_not_zero() {
                                let square = match squares.lsb_index() {
                                    Some(x) => x,
                                    None => unreachable!(),
                                };

                                if chessboard.is_square_attacked(square, side.update()) {
                                    can_castle = false;
                                }
                                squares = squares.pop_bit(square);
                            }

                            if can_castle {
                                arr = match side {
                                    Side::White => arr.append_one_move(03, 01, None, MoveType::Castle),
                                    Side::Black => arr.append_one_move(59, 57, None, MoveType::Castle),
                                }
                            }
                        }

                        let (q_mask, q_index) = match side {
                            Side::White => (W_QUEEN_SIDE_CASTLE_MASK, 1),
                            Side::Black => (B_QUEEN_SIDE_CASTLE_MASK, 3),
                        };
                        // queen side
                        if chessboard.castle_bools[q_index] && blockers.bit_and(&q_mask).is_zero() {
                            //check if squares are under attack
                            let mut squares = match side {
                                Side::White => q_mask.bit_and(&BB::nth(06).bit_not()), //this is where the white queen side rook is
                                Side::Black => q_mask.bit_and(&BB::nth(64).bit_not()), //this is where the black queen side rook is
                            };
                            let mut can_castle = true;

                            while squares.is_not_zero() {
                                let square = match squares.lsb_index() {
                                    Some(x) => x,
                                    None => unreachable!(),
                                };

                                if chessboard.is_square_attacked(square, side.update()) {
                                    can_castle = false;
                                }
                                squares = squares.pop_bit(square);
                            }
                            if can_castle {
                                arr = match side {
                                    Side::White => arr.append_one_move(03, 05, None, MoveType::Castle),
                                    Side::Black => arr.append_one_move(59, 61, None, MoveType::Castle),
                                }
                            }
                        }
                    }

                    /* moves and attacks */
                    let mut attacks = get_king_attack(source).bit_and(&friends);
                    while attacks.is_not_zero() {
                        let target = match attacks.lsb_index() {
                            Some(x) => x,
                            None => unreachable!(),
                        };
                        // king cannot move to a square under attack
                        let mut removed_king_cb = chessboard.const_clone();
                        let king_index = match side {
                            Side::White => 0,
                            Side::Black => 6,
                        };
                        removed_king_cb.piece_bbs[king_index] = BB::ZERO;
                        removed_king_cb.mailbox[king_index] = None;
                        if !removed_king_cb.is_square_attacked(target, side.update()) {
                            arr = arr.append_one_move(source, target, None, MoveType::Normal);
                        };
                        attacks = attacks.pop_bit(target);
                    }
                }

                /* queen */
                01 | 07 => {
                    let mut attacks = get_queen_attack(source, blockers).bit_and(&friends.bit_not());
                    while attacks.is_not_zero() {
                        let target = match attacks.lsb_index() {
                            Some(x) => x,
                            None => unreachable!(),
                        };

                        // only consider moves along pinning ray if pinned
                        if pin_mask.is_not_zero() && pin_mask.nth_is_zero(target) {
                            attacks = attacks.pop_bit(target);
                            continue;
                        }

                        // only consider moves along checking ray if in check
                        if check_mask.is_not_zero() && check_mask.nth_is_zero(target) {
                            attacks = attacks.pop_bit(target);
                            continue;
                        }

                        // when double checked king has to move
                        if checkers_count > 1 {
                            attacks = attacks.pop_bit(target);
                            continue;
                        }

                        arr = arr.append_one_move(source, target, None, MoveType::Normal);
                        attacks = attacks.pop_bit(target);
                    }
                }

                /* knights */
                02 | 08 => {
                    let mut attacks = get_knight_attack(source).bit_and(&friends.bit_not());
                    // pinned knights can not move
                    if pin_mask.is_not_zero() {
                        sources = sources.pop_bit(source);
                        continue;
                    }

                    while attacks.is_not_zero() {
                        let target = match attacks.lsb_index() {
                            Some(x) => x,
                            None => unreachable!(),
                        };

                        // only consider moves along checking ray if in check
                        if check_mask.is_not_zero() && check_mask.nth_is_zero(target) {
                            attacks = attacks.pop_bit(target);
                            continue;
                        }

                        // when double checked king has to move
                        if checkers_count > 1 {
                            attacks = attacks.pop_bit(target);
                            continue;
                        }

                        arr = arr.append_one_move(source, target, None, MoveType::Normal);
                        attacks = attacks.pop_bit(target);
                    }
                }

                /* bishops */
                03 | 09 => {
                    let mut attacks = get_bishop_attack(source, blockers).bit_and(&friends.bit_not());
                    while attacks.is_not_zero() {
                        let target = match attacks.lsb_index() {
                            Some(x) => x,
                            None => unreachable!(),
                        };

                        // only consider moves along pinning ray if pinned
                        if pin_mask.is_not_zero() && pin_mask.nth_is_zero(target) {
                            attacks = attacks.pop_bit(target);
                            continue;
                        }

                        // only consider moves along checking ray if in check
                        if check_mask.is_not_zero() && check_mask.nth_is_not_zero(target) {
                            attacks = attacks.pop_bit(target);
                            continue;
                        }

                        // when double checked king has to move
                        if checkers_count > 1 {
                            attacks = attacks.pop_bit(target);
                            continue;
                        }

                        arr = arr.append_one_move(source, target, None, MoveType::Normal);
                        attacks = attacks.pop_bit(target);
                    }
                }

                /* rooks */
                04 | 10 => {
                    let mut attacks = get_rook_attack(source, blockers).bit_and(&friends.bit_not());
                    while attacks.is_not_zero() {
                        let target = match attacks.lsb_index() {
                            Some(x) => x,
                            None => unreachable!(),
                        };

                        // only consider moves along pinning ray if pinned
                        if pin_mask.is_not_zero() && pin_mask.nth_is_zero(target) {
                            attacks = attacks.pop_bit(target);
                            continue;
                        }

                        // only consider moves along checking ray if in check
                        if check_mask.is_zero() && check_mask.nth_is_zero(target) {
                            attacks = attacks.pop_bit(target);
                            continue;
                        }

                        // when double checked king has to move
                        if checkers_count > 1 {
                            attacks = attacks.pop_bit(target);
                            continue;
                        }

                        arr = arr.append_one_move(source, target, None, MoveType::Normal);
                        attacks = attacks.pop_bit(target);
                    }
                }

                /* pawns */
                05 | 11 => {
                    let mut is_diagonal_pinned = false;
                    let mut is_vertical_pinned = false;
                    let mut is_horizontal_pinned = false;

                    if pin_mask.is_not_zero() {
                        // TODO: FIX HERE!!! slow?
                        let mut squares = pinners;
                        while squares.is_not_zero() {
                            let square = match squares.lsb_index() {
                                Some(x) => x,
                                None => unreachable!(),
                            };
                            assert!(source != square);
                            if RAYS[king_pos][square].nth_is_not_zero(source) {
                                if DDIAG[source] == DDIAG[square] || ADIAG[source] == ADIAG[square] {
                                    is_diagonal_pinned = true;
                                } else if COLS[source] == COLS[square] {
                                    is_vertical_pinned = true;
                                } else if ROWS[source] == ROWS[square] {
                                    is_horizontal_pinned = true;
                                }
                            }
                            squares = squares.pop_bit(square);
                        }
                    }

                    /* pawn moves */
                    if !is_diagonal_pinned && !is_horizontal_pinned {
                        let target = match side {
                            Side::White => source + 8,
                            Side::Black => source - 8,
                        };
                        // can only move one square if next square is empty
                        if blockers.nth_is_zero(target) {
                            // can only move one square if not in check or blocks check
                            if check_mask.is_zero() || (check_mask.nth_is_not_zero(target) && checkers_count == 1) {
                                let next_square_promotion = match side {
                                    Side::White => source >= 48,
                                    Side::Black => source <= 15,
                                };

                                if next_square_promotion {
                                    // promotions
                                    arr = arr.new_promotions(source, target);
                                } else {
                                    // pawn move 1 square
                                    arr = arr.append_one_move(source, target, None, MoveType::Normal);
                                }
                            }
                        }

                        /* two square */
                        let next = match side {
                            Side::White => source + 8,
                            Side::Black => source - 8,
                        };

                        let is_initial_sq = match side {
                            Side::White => ROWS[source] == 1,
                            Side::Black => ROWS[source] == 6,
                        };
                        if is_initial_sq {
                            let target = match side {
                                Side::White => source + 16,
                                Side::Black => source - 16,
                            };
                            // can only move two squares if pawn is at starting position, and next two squares are empty
                            if blockers.bit_and(&BB::nth(next).bit_or(&BB::nth(target))).is_zero() {
                                // can only move one square if not in check or blocks check
                                if check_mask.is_zero() || (check_mask.nth_is_not_zero(target) && checkers_count == 1) {
                                    arr = arr.append_one_move(source, target, None, MoveType::Normal);
                                }
                            }
                        }
                    }

                    /* pawn attacks */
                    if !is_horizontal_pinned && !is_vertical_pinned {
                        let mut attacks = match side {
                            Side::White => get_w_pawn_attack(source).bit_and(&b_blockers),
                            Side::Black => get_b_pawn_attack(source).bit_and(&w_blockers),
                        };

                        while attacks.is_not_zero() {
                            let target = match attacks.lsb_index() {
                                Some(x) => x,
                                None => unreachable!(),
                            };

                            // can only attack a square if not in check or attack blocks check
                            if check_mask.is_zero() || (check_mask.nth_is_not_zero(target) && checkers_count == 1) {
                                //can only attack a square if not pinned or attack is along pin ray
                                if pin_mask.is_zero() || pin_mask.nth_is_not_zero(target) {
                                    let next_square_promotion = match side {
                                        Side::White => source >= 48,
                                        Side::Black => source <= 15,
                                    };

                                    if next_square_promotion {
                                        // promotions
                                        arr = arr.new_promotions(source, target);
                                    } else {
                                        // pawn capture
                                        arr = arr.append_one_move(source, target, None, MoveType::Normal);
                                    }
                                }
                            }
                            attacks = attacks.pop_bit(target);
                        }
                    }

                    /* en passant */
                    if chessboard.enpassant_bb.is_not_zero() && !is_pinned {
                        let mut targets = match side {
                            Side::White => chessboard.enpassant_bb.bit_and(&get_w_pawn_attack(source)),
                            Side::Black => chessboard.enpassant_bb.bit_and(&get_b_pawn_attack(source)),
                        };
                        while targets.is_not_zero() {
                            let target = match targets.lsb_index() {
                                Some(x) => x,
                                None => unreachable!(),
                            };

                            // special psuedo-pinned pawn case:
                            // R . p P k
                            // . . . ^ .
                            // . . . | .
                            // . . . x .

                            let row_bb = BB::new(0b11111111u64 << (8 * ROWS[source]));

                            //(enemy rook, enemy pawn, enemy pawn position)
                            let (r_index, p_index, p_pos) = match side {
                                Side::White => (10, 11, target - 8),
                                Side::Black => (04, 05, target + 8),
                            };

                            // if enemy rook and friendly king is in the same row, check for special case
                            if (ROWS[king_pos] == ROWS[source]) && (chessboard.piece_bbs[r_index].bit_and(&row_bb).is_not_zero()) {
                                // check if enpassant leaves king in check
                                let mut test_cb = chessboard.const_clone();
                                test_cb.piece_bbs[i] = test_cb.piece_bbs[i].bit_and(&BB::nth(source).bit_not());
                                test_cb.piece_bbs[i] = test_cb.piece_bbs[i].bit_and(&BB::nth(target));
                                test_cb.piece_bbs[p_index] = test_cb.piece_bbs[p_index].bit_and(&BB::nth(p_pos).bit_not());

                                if test_cb.king_is_in_check(side) {
                                    targets = targets.pop_bit(target);
                                    continue;
                                }
                            }

                            // if there are no checks
                            if chessboard.check_bb.is_zero() {
                                arr = arr.append_one_move(source, target, None, MoveType::EnPassant);
                                targets = targets.pop_bit(target);
                                continue;
                            }

                            // if in check, can only en passant to remove checking pawn
                            if checkers_count == 1 {
                                let checker = match chessboard.check_bb.lsb_index() {
                                    Some(x) => x,
                                    None => unreachable!(),
                                };

                                let enemy_pawn_pos = match side {
                                    Side::White => target - 8,
                                    Side::Black => target + 8,
                                };

                                if checker == enemy_pawn_pos {
                                    arr = arr.append_one_move(source, target, None, MoveType::EnPassant);
                                }
                            }
                            targets = targets.pop_bit(target);
                        }
                    }
                }

                _ => unreachable!(),
            }
            sources = sources.pop_bit(source);
        }
        i += 1;
    }
    arr
}

//pub const fn get_history(&self) -> [([BitBoard; 12], u16); 8] {
//    let mut new = self.history;
//    let mut i = 0;
//    while i < 7 {
//        new[i] = self.history[i + 1];
//        i += 1;
//    }
//    new[7] = (self.piece_bbs, self.rep_count());
//
//    return new;
//}
