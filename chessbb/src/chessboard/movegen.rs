use crate::{
    Bitboard, ChessBoard, ChessMove, ChessPiece, PieceType, Side,
    bitboard::attack::{
        get_bishop_attack, get_king_attack, get_knight_attack, get_pawn_attack, get_pawn_quiet, get_queen_attack, get_rook_attack, get_rook_ray, long_rays,
        promotion_row, starting_row,
    },
    chessboard::MoveList,
    chessmove::{Castling, MoveType},
    chesspiece::SliderType,
    square::Square,
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

        //let side: Side = self.data.side_to_move;
        //#[cfg(feature = "kinglessattackmask")]
        //let kingless_blockers: Bitboard = self.bitboards.blockers().bit_xor(&self.bitboards.piece_bitboard(ChessPiece(side, PieceType::King)));
        //#[cfg(feature = "kinglessattackmask")]
        //let kingless_attack_mask: Bitboard = self.calculate_attacked_mask(kingless_blockers);

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
    fn _pawn_moves(&self, moves: &mut MoveList) {
        let side = self.data.side_to_move;
        let blockers = self.bitboards.blockers();
        let check_mask = self.data.check_mask;
        let king_square = self.bitboards.piece_bitboard(ChessPiece(side, PieceType::King)).lsb_square().expect("King not found!");

        let pawns = self.bitboards.piece_bitboard(ChessPiece(side, PieceType::Pawn));
        let attacking_pawns = self.calculate_attacking_pawns();
        let mut pinned_pawns = pawns.bit_and(&self.data.pinned_bb);
        let mut non_pinned_pawns = pawns.bit_xor(&pinned_pawns);
        let mut non_pinned_attacking_pawns = non_pinned_pawns.bit_and(&attacking_pawns); //subset of non_pinned_pawns

        //println!("attacking_pawns:\n{}", attacking_pawns);
        //println!("pinned_pawns:\n{}", pinned_pawns);
        //println!("non_pinned_pawns:\n{}", non_pinned_pawns);
        //println!("non_pinned_attacking_pawns:\n{}", non_pinned_attacking_pawns);

        while non_pinned_pawns.is_not_zero() {
            let source = non_pinned_pawns.lsb_square().unwrap();

            let single_square = match side {
                Side::White => source.up(),
                Side::Black => source.down(),
            };

            /* pawn move - one square */

            // can only move one square if next square is empty
            if blockers.nth_is_zero(single_square) {
                debug_assert!(self.data.check_bb.count_ones() <= 1);
                // can only move one-square if not in check, or blocks check
                if check_mask.is_zero() || check_mask.nth_is_not_zero(single_square) {
                    match single_square.to_row_usize() == promotion_row(side) {
                        #[cfg(feature = "arrayvec")]
                        //safe because: https://lichess.org/@/Tobs40/blog/why-a-position-cant-have-more-than-218-moves/a5xdxeqs
                        true => unsafe { moves.try_extend_from_slice(&ChessMove::promotions(source, single_square)).unwrap_unchecked() },

                        #[cfg(not(any(feature = "arrayvec")))]
                        true => moves.extend_from_slice(&ChessMove::promotions(source, target)),

                        false => moves.push(ChessMove::new(source, single_square, MoveType::Normal)),
                    }
                }
            }

            /* pawn move - two squares */

            //can only move two-squares if pawn is in starting row, and next two squares are empty
            if source.to_row_usize() == starting_row(side) {
                let double_square = match side {
                    Side::White => source.upup(),
                    Side::Black => source.downdown(),
                };
                if blockers.bit_and(&Bitboard::nth(single_square).bit_or(&Bitboard::nth(double_square))).is_zero() {
                    // can only move two-squares if not in check, or blocks check
                    if check_mask.is_zero() || check_mask.nth_is_not_zero(double_square) {
                        moves.push(ChessMove::new(source, double_square, MoveType::Normal));
                    }
                }
            }

            non_pinned_pawns.pop_lsb();
        }

        'attacking_pawns: while non_pinned_attacking_pawns.is_not_zero() {
            let source = non_pinned_attacking_pawns.lsb_square().unwrap();
            let mut attacks = get_pawn_attack(side, source).bit_and(&self.bitboards.colour_blockers(side.update()));
            /* pawn attack - normal */
            while attacks.is_not_zero() {
                let attack = attacks.lsb_square().unwrap();
                debug_assert!(self.data.check_bb.count_ones() <= 1);
                //can only attack a square if not in check or attack blocks check
                if check_mask.is_zero() || check_mask.nth_is_not_zero(attack) {
                    match attack.to_row_usize() == promotion_row(side) {
                        #[cfg(feature = "arrayvec")]
                        //safe because: https://lichess.org/@/Tobs40/blog/why-a-position-cant-have-more-than-218-moves/a5xdxeqs
                        true => unsafe { moves.try_extend_from_slice(&ChessMove::promotions(source, attack)).unwrap_unchecked() },

                        #[cfg(not(any(feature = "arrayvec")))]
                        true => moves.extend_from_slice(&mut ChessMove::promotions(source, attack)),

                        false => moves.push(ChessMove::new(source, attack, MoveType::Normal)),
                    }
                }
                attacks.pop_lsb();
            }

            /* pawn attack - enpassant */
            if let Some(enpassant_square) = self.data.enpassant_bb.lsb_square() {
                if get_pawn_attack(side, source).nth_is_not_zero(enpassant_square) {
                    let enemy_pawn_square = match side {
                        Side::White => enpassant_square.down(),
                        Side::Black => enpassant_square.up(),
                    };

                    //if (enemy rook OR enemy queen) AND friendly king AND friendly pawn is in the same row, check for special case
                    if Square::is_same_row(source, king_square) {
                        let enemy_side = side.update();
                        let row_bb = Bitboard::rows(source.to_row_usize()).bit_and(&Bitboard::rows(king_square.to_row_usize()));
                        let enemy_rook_or_queen = self
                            .bitboards
                            .piece_bitboard(ChessPiece(enemy_side, PieceType::Rook))
                            .bit_or(&self.bitboards.piece_bitboard(ChessPiece(enemy_side, PieceType::Queen)));
                        if row_bb.bit_and(&enemy_rook_or_queen).is_not_zero() {
                            //cozy-chess tech
                            let resulting_blockers =
                                self.bitboards.blockers() ^ Bitboard::nth(source) ^ Bitboard::nth(enemy_pawn_square) | Bitboard::nth(enpassant_square);
                            if (get_rook_attack(king_square, resulting_blockers) & enemy_rook_or_queen).is_not_zero() {
                                non_pinned_attacking_pawns.pop_lsb();
                                continue;
                            }
                        }
                    }

                    //if in check, can only enpassant to remove checking pawn
                    if self.data.check_bb.count_ones() == 1 {
                        let checker_square = self.data.check_bb.lsb_square().unwrap();
                        if checker_square == enemy_pawn_square {
                            moves.push(ChessMove::new(source, enpassant_square, MoveType::EnPassant));
                        }

                        non_pinned_attacking_pawns.pop_lsb();
                        continue 'attacking_pawns;
                    }

                    //if there are no checks
                    moves.push(ChessMove::new(source, enpassant_square, MoveType::EnPassant));
                }
            }

            non_pinned_attacking_pawns.pop_lsb();
        }

        'pinned_pawns: while pinned_pawns.is_not_zero() {
            let source = pinned_pawns.lsb_square().unwrap();
            let pin_mask = self.pin_mask(source);
            let pinners = self.data.pinner_bb;

            let mut is_pinned_diag: bool = false;
            let mut is_pinned_vert: bool = false;
            let mut is_pinned_horz: bool = false;

            if pin_mask.is_not_zero() {
                let mut pinners = self.data.pinner_bb;
                while pinners.is_not_zero() {
                    let pinner = pinners.lsb_square().unwrap();
                    let piece_type = self.mailbox.square_index(pinner).unwrap();

                    is_pinned_diag |= Square::is_same_diag(source, pinner, king_square)
                        && matches!(piece_type, ChessPiece(_, PieceType::Bishop) | ChessPiece(_, PieceType::Queen));
                    is_pinned_vert |= Square::is_same_col(source, pinner)
                        && Square::is_same_col(pinner, king_square)
                        && matches!(piece_type, ChessPiece(_, PieceType::Rook) | ChessPiece(_, PieceType::Queen));
                    is_pinned_horz |= Square::is_same_row(source, pinner)
                        && Square::is_same_row(pinner, king_square)
                        && matches!(piece_type, ChessPiece(_, PieceType::Rook) | ChessPiece(_, PieceType::Queen));
                    pinners.pop_lsb();
                }
            }

            if !is_pinned_diag && !is_pinned_horz {
                let single_square = match side {
                    Side::White => source.up(),
                    Side::Black => source.down(),
                };

                /* pawn move - one square */

                // can only move one square if next square is empty
                if blockers.nth_is_zero(single_square) {
                    debug_assert!(self.data.check_bb.count_ones() <= 1);
                    // can only move one-square if not in check, or blocks check
                    if check_mask.is_zero() || check_mask.nth_is_not_zero(single_square) {
                        match single_square.to_row_usize() == promotion_row(side) {
                            #[cfg(feature = "arrayvec")]
                            //safe because: https://lichess.org/@/Tobs40/blog/why-a-position-cant-have-more-than-218-moves/a5xdxeqs
                            true => unsafe { moves.try_extend_from_slice(&ChessMove::promotions(source, single_square)).unwrap_unchecked() },

                            #[cfg(not(any(feature = "arrayvec")))]
                            true => moves.extend_from_slice(&ChessMove::promotions(source, target)),

                            false => moves.push(ChessMove::new(source, single_square, MoveType::Normal)),
                        }
                    }
                }

                /* pawn move - two squares */

                //can only move two-squares if pawn is in starting row, and next two squares are empty
                if source.to_row_usize() == starting_row(side) {
                    let double_square = match side {
                        Side::White => source.upup(),
                        Side::Black => source.downdown(),
                    };
                    if blockers.bit_and(&Bitboard::nth(single_square).bit_or(&Bitboard::nth(double_square))).is_zero() {
                        // can only move two-squares if not in check, or blocks check
                        if check_mask.is_zero() || check_mask.nth_is_not_zero(double_square) {
                            moves.push(ChessMove::new(source, double_square, MoveType::Normal));
                        }
                    }
                }
            }

            /* pawn attack - normal */
            if attacking_pawns.nth_is_not_zero(source) && !is_pinned_horz && !is_pinned_vert {
                let mut attacks = get_pawn_attack(side, source).bit_and(&self.bitboards.colour_blockers(side.update()));

                while attacks.is_not_zero() {
                    let attack = attacks.lsb_square().unwrap();
                    let attack_bb = attacks.lsb_bitboard();
                    debug_assert!(self.data.check_bb.count_ones() <= 1);
                    //can only attack a square if not in check or attack blocks check
                    if check_mask.is_zero() || check_mask.bit_and(&attack_bb).is_not_zero() {
                        let is_attack_pinner = pinners.bit_and(&attack_bb).is_not_zero() && Square::is_same_diag(source, attack, king_square);

                        //can only attack a square if not pinned or capturing piece pinning the pawn
                        if pin_mask.is_zero() || is_attack_pinner {
                            match attack.to_row_usize() == promotion_row(side) {
                                #[cfg(feature = "arrayvec")]
                                //safe because: https://lichess.org/@/Tobs40/blog/why-a-position-cant-have-more-than-218-moves/a5xdxeqs
                                true => unsafe { moves.try_extend_from_slice(&ChessMove::promotions(source, attack)).unwrap_unchecked() },

                                #[cfg(not(any(feature = "arrayvec")))]
                                true => moves.extend_from_slice(&mut ChessMove::promotions(source, attack)),

                                false => moves.push(ChessMove::new(source, attack, MoveType::Normal)),
                            }
                        }
                    }
                    attacks.pop_lsb();
                }

                /* pawn attack - enpassant */
                if let Some(enpassant_square) = self.data.enpassant_bb.lsb_square() {
                    if get_pawn_attack(side, source).nth_is_not_zero(enpassant_square) {
                        let enemy_pawn_square = match side {
                            Side::White => enpassant_square.down(),
                            Side::Black => enpassant_square.up(),
                        };

                        //if (enemy rook OR enemy queen) AND friendly king AND friendly pawn is in the same row, check for special case
                        if Square::is_same_row(source, king_square) {
                            let enemy_side = side.update();
                            let row_bb = Bitboard::rows(source.to_row_usize()).bit_and(&Bitboard::rows(king_square.to_row_usize()));
                            let enemy_rook_or_queen = self
                                .bitboards
                                .piece_bitboard(ChessPiece(enemy_side, PieceType::Rook))
                                .bit_or(&self.bitboards.piece_bitboard(ChessPiece(enemy_side, PieceType::Queen)));
                            if row_bb.bit_and(&enemy_rook_or_queen).is_not_zero() {
                                //cozy-chess tech
                                let resulting_blockers =
                                    self.bitboards.blockers() ^ Bitboard::nth(source) ^ Bitboard::nth(enemy_pawn_square) | Bitboard::nth(enpassant_square);
                                if (get_rook_attack(king_square, resulting_blockers) & enemy_rook_or_queen).is_not_zero() {
                                    pinned_pawns.pop_lsb();
                                    continue;
                                }
                            }
                        }

                        //if in check, can only enpassant to remove checking pawn
                        if self.data.check_bb.count_ones() == 1 {
                            let checker_square = self.data.check_bb.lsb_square().unwrap();
                            if checker_square == enemy_pawn_square {
                                moves.push(ChessMove::new(source, enpassant_square, MoveType::EnPassant));
                            }

                            pinned_pawns.pop_lsb();
                            continue 'pinned_pawns;
                        }

                        //if pinned diagonally, can only enpassant
                        if is_pinned_diag && pin_mask.nth_is_zero(enpassant_square) {
                            pinned_pawns.pop_lsb();
                            continue 'pinned_pawns;
                        }

                        //if there are no checks
                        moves.push(ChessMove::new(source, enpassant_square, MoveType::EnPassant));
                    }
                }
            }
            pinned_pawns.pop_lsb();
        }
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
            let targets = (get_pawn_quiedddt(side, source, &blockers) | (get_pawn_attack(side, source) & enemies)) & *target_squares;
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

                let row_bb = Bitboard::rows(source.to_row_usize()).bit_and(&Bitboard::rows(king_square.to_row_usize()));
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
