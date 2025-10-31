use crate::{
    Bitboard, ChessBoard, ChessMove, ChessPiece, PieceType, Side,
    bitboard::attack::{get_bishop_attack, get_king_attack, get_knight_attack, get_pawn_attack, get_pawn_quiet, get_queen_attack, get_rook_attack, long_rays},
    chessboard::MoveList,
    chessmove::{Castling, MoveType},
    chesspiece::SliderType,
};

#[cfg(feature = "arrayvec")]
use arrayvec::ArrayVec;

#[cfg(feature = "smallvec")]
use smallvec::SmallVec;

impl ChessBoard {
    pub fn generate_moves(&self) -> MoveList {
        #[cfg(feature = "arrayvec")]
        let mut moves: MoveList = ArrayVec::new();

        #[cfg(feature = "smallvec")]
        let mut moves: MoveList = SmallVec::with_capacity(64);

        #[cfg(not(any(feature = "arrayvec", feature = "smallvec")))]
        let mut moves: MoveList = Vec::with_capacity(40);

        // consider if king is in check
        let checkers_count: u32 = self.data.check_bb.count_ones();

        match checkers_count {
            0 => {
                let target_mask = self.target_mask::<false>();
                self.pawn_moves::<false>(&mut moves, &target_mask);
                self.knight_moves(&mut moves, &target_mask);
                self.slider_moves::<false>(&mut moves, SliderType::Bishop, &target_mask);
                self.slider_moves::<false>(&mut moves, SliderType::Rook, &target_mask);
                self.slider_moves::<false>(&mut moves, SliderType::Queen, &target_mask);
                self.king_moves::<false>(&mut moves)
            }

            1 => {
                let target_mask = self.target_mask::<true>();
                self.pawn_moves::<true>(&mut moves, &target_mask);
                self.knight_moves(&mut moves, &target_mask);
                self.slider_moves::<true>(&mut moves, SliderType::Bishop, &target_mask);
                self.slider_moves::<true>(&mut moves, SliderType::Rook, &target_mask);
                self.slider_moves::<true>(&mut moves, SliderType::Queen, &target_mask);
                self.king_moves::<true>(&mut moves)
            }

            _ => self.king_moves::<true>(&mut moves),
        }

        return moves;
    }

    //from cozy-chess
    const fn target_mask<const IS_IN_CHECK: bool>(&self) -> Bitboard {
        debug_assert!(self.data.check_bb.count_ones() < 2);
        let side = self.side();
        let targets = match IS_IN_CHECK {
            true => self.data.check_mask,
            false => Bitboard::ONES,
        };

        return targets.bit_and(&self.bitboards.colour_blockers(side).bit_not());
    }

    fn pawn_moves<const IS_IN_CHECK: bool>(&self, moves: &mut MoveList, target_squares: &Bitboard) {
        //function assumes not in double check
        debug_assert!(self.data.check_bb.count_ones() < 2);
        let side = self.side();
        let enemy_side = side.update();
        let pawns = self.bitboards.piece_bitboard(ChessPiece(side, PieceType::Pawn));
        let pinned = self.data.pinned_bb;
        let blockers = self.bitboards.blockers();
        let enemies = self.bitboards.colour_blockers(enemy_side);
        let king_square = self.king_square(side);
        for source in pawns & !pinned {
            let targets = (get_pawn_quiet(side, source, &blockers) | (get_pawn_attack(side, source) & enemies)) & *target_squares;
            ChessMove::add_pawn_moves(source, targets, moves);
        }

        if !IS_IN_CHECK {
            for source in pawns & pinned {
                let target_squares = target_squares.bit_and(&long_rays(king_square, source));
                let targets = (get_pawn_quiet(side, source, &blockers) | (get_pawn_attack(side, source) & enemies)) & target_squares;
                ChessMove::add_pawn_moves(source, targets, moves);
            }
        }

        if let Some(enpassant_square) = self.data.enpassant_bb.lsb_square() {
            let enemy_pawn_square = match side {
                Side::White => enpassant_square.down(),
                Side::Black => enpassant_square.up(),
            };
            //println!("sources:\n{}", get_pawn_attack(side.update(), enpassant_square) & pawns);
            for source in get_pawn_attack(side.update(), enpassant_square) & pawns {
                let target_squares = match pinned.nth_is_not_zero(source) {
                    true => target_squares.bit_and(&long_rays(king_square, source)),
                    false => *target_squares,
                };
                //println!("targets:\n{}", (get_pawn_attack(side, source) & self.data.enpassant_bb) & target_squares);
                if !IS_IN_CHECK {
                    let targets = (get_pawn_attack(side, source) & self.data.enpassant_bb) & target_squares;
                    if targets.nth_is_zero(enpassant_square) {
                        continue;
                    }
                } else {
                    if (self.data.check_bb.lsb_square().unwrap() != enemy_pawn_square) || self.data.pinned_bb.nth_is_not_zero(source) {
                        continue;
                    }
                }

                let row_bb = Bitboard::rows(source.as_row_usize()).bit_and(&Bitboard::rows(king_square.as_row_usize()));
                let horizontal_attackers = self.bitboards.piece_bitboard(ChessPiece(enemy_side, PieceType::Rook))
                    | self.bitboards.piece_bitboard(ChessPiece(enemy_side, PieceType::Queen));

                if row_bb.bit_and(&horizontal_attackers).is_not_zero() {
                    //cozy-chess tech
                    let resulting_blockers =
                        self.bitboards.blockers() ^ Bitboard::nth(source) ^ Bitboard::nth(enemy_pawn_square) | Bitboard::nth(enpassant_square);
                    if (get_rook_attack(king_square, resulting_blockers) & horizontal_attackers).is_not_zero() {
                        continue;
                    }
                }

                //if in check, can only enpassant to remove checking pawn
                if IS_IN_CHECK {
                    if self.data.check_bb.nth_is_not_zero(enemy_pawn_square) {
                        moves.push(ChessMove::new(source, enpassant_square, MoveType::EnPassant));
                    }
                    continue;
                }
                moves.push(ChessMove::new(source, enpassant_square, MoveType::EnPassant));
            }
        }
        //
    }

    fn knight_moves(&self, moves: &mut MoveList, target_squares: &Bitboard) {
        debug_assert!(self.data.check_bb.count_ones() < 2);
        let side = self.side();
        let pinned = self.data.pinned_bb;
        let knights = self.bitboards.piece_bitboard(ChessPiece(side, PieceType::Knight));

        for source in knights & !pinned {
            let targets = get_knight_attack(source).bit_and(target_squares);
            ChessMove::add_normal_moves(source, targets, moves);
        }
    }

    fn slider_moves<const IS_IN_CHECK: bool>(&self, moves: &mut MoveList, piece: SliderType, target_squares: &Bitboard) {
        debug_assert!(self.data.check_bb.count_ones() < 2);
        let side = self.side();
        let pinned = self.data.pinned_bb;
        let sliders = self.bitboards.piece_bitboard(ChessPiece(side, PieceType::from(piece)));
        let blockers = self.bitboards.blockers();

        for source in sliders & !pinned {
            let targets = piece.get_attack(source, blockers) & *target_squares;
            ChessMove::add_normal_moves(source, targets, moves);
        }

        if !IS_IN_CHECK {
            let king_square = self.king_square(side);

            for source in sliders & pinned {
                let targets = piece.get_attack(source, blockers) & (*target_squares & long_rays(king_square, source));
                ChessMove::add_normal_moves(source, targets, moves);
            }
        }
    }

    fn king_moves<const IS_IN_CHECK: bool>(&self, moves: &mut MoveList) {
        let side = self.side();
        let king_square = self.king_square(side);
        let kingless_blockers = self.bitboards.blockers().bit_xor(&self.bitboards.piece_bitboard(ChessPiece(side, PieceType::King)));
        #[cfg(feature = "kinglessattackmask")]
        let mask = self.calculate_attacked_mask(kingless_blockers);

        #[cfg(not(feature = "kinglessattackmask"))]
        for target in get_king_attack(king_square) & !self.bitboards.colour_blockers(side) {
            if !self.is_square_attacked(target, side.update(), kingless_blockers) {
                moves.push(ChessMove::new(king_square, target, MoveType::Normal));
            }
        }

        #[cfg(feature = "kinglessattackmask")]
        ChessMove::add_normal_moves(king_square, get_king_attack(king_square) & !self.bitboards.colour_blockers(side) & !mask, moves);

        if !IS_IN_CHECK {
            if self.is_castling_legal(Castling::Kingside(side)) {
                moves.push(ChessMove::kingside_castle(side));
            }
            if self.is_castling_legal(Castling::Queenside(side)) {
                moves.push(ChessMove::queenside_castle(side));
            }
        }
    }

    fn calculate_attacked_mask(&self, blockers: Bitboard) -> Bitboard {
        let enemy_side = self.side().update();
        let mut attack_mask: Bitboard = self.calculate_pawn_attack_mask(enemy_side);

        for &piece in PieceType::iter() {
            if piece == PieceType::Pawn {
                continue;
            }
            let mut attackers = self.bitboards.piece_bitboard(ChessPiece(enemy_side, piece));

            while attackers.is_not_zero() {
                let attacker = attackers.lsb_square().unwrap();
                attack_mask |= match piece {
                    PieceType::Pawn => unreachable!(),
                    PieceType::Knight => get_knight_attack(attacker),
                    PieceType::Bishop => get_bishop_attack(attacker, blockers),
                    PieceType::Rook => get_rook_attack(attacker, blockers),
                    PieceType::Queen => get_queen_attack(attacker, blockers),
                    PieceType::King => get_king_attack(attacker),
                };
                attackers.pop_lsb();
            }
        }

        //println!("attack_mask:\n{}", attack_mask);

        return attack_mask;
    }

    const fn calculate_attacking_pawns(&self) -> Bitboard {
        let side = self.data.side_to_move;
        let targets = self.bitboards.colour_blockers(side.update()).bit_or(&self.data.enpassant_bb);

        return match side {
            Side::White => (targets.shr(9).bit_and(&Bitboard::NOT_A_FILE)).bit_or(&targets.shr(7).bit_and(&Bitboard::NOT_H_FILE)),
            Side::Black => (targets.shl(9).bit_and(&Bitboard::NOT_H_FILE)).bit_or(&targets.shl(7).bit_and(&Bitboard::NOT_A_FILE)),
        }
        .bit_and(&self.bitboards.piece_bitboard(ChessPiece(side, PieceType::Pawn)));
    }

    const fn calculate_pawn_attack_mask(&self, side: Side) -> Bitboard {
        let pawn_bb = self.bitboards.piece_bitboard(ChessPiece(side, PieceType::Pawn));

        return match side {
            Side::White => (pawn_bb.shl(9).bit_and(&Bitboard::NOT_H_FILE)).bit_or(&pawn_bb.shl(7).bit_and(&Bitboard::NOT_A_FILE)),
            Side::Black => (pawn_bb.shr(9).bit_and(&Bitboard::NOT_A_FILE)).bit_or(&pawn_bb.shr(7).bit_and(&Bitboard::NOT_H_FILE)),
        };
    }
}
