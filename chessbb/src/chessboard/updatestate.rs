use crate::{
    Bitboard, ChessBoard, ChessMove, ChessPiece, PieceType, Side,
    bitboard::attack::{get_b_pawn_attack, get_bishop_ray, get_knight_attack, get_rook_ray, get_w_pawn_attack, rays},
    chessboard::zobrist::ZobristHash,
    chessmove::{Castling, MoveType},
    square::Square,
};

impl ChessBoard {
    pub fn update_state(&mut self, chess_move: &ChessMove) {
        let mut enpassant_bb: Bitboard = Bitboard::ZERO;
        let mut check_bb: Bitboard = Bitboard::ZERO;
        let mut pinned_bb: Bitboard = Bitboard::ZERO;
        let mut pinner_bb: Bitboard = Bitboard::ZERO;
        let side = self.side();
        let enm_king_square: Square = self.bitboards.piece_bitboard(ChessPiece(side.update(), PieceType::King)).lsb_square().expect("King not found!");
        let source: Square = chess_move.source();
        let target: Square = chess_move.target();
        //assert!(
        //    self.mailbox.square_index(&source).expect("update_state error: source mailbox is None");.is_some(),
        //    "position:\n\r{}\n\rposition:\n\r{}\n\rchess_move:{:?}\n\rchess_move:{:?}\n\r",
        //    self,
        //    self,
        //    chess_move,
        //    chess_move
        //);
        let source_piece = self.mailbox.square_index(source).expect("update_state error: source mailbox is None");
        let target_piece = self.mailbox.square_index(target);

        //assert!(self.piece_bbs[enemy_king_index].nth_is_zero(target), "position:\n\r{}\n\rposition:\n\r{}\n\rposition:\n\r{}\n\r", self, self, self);
        let mut current_hash = self.hash();
        current_hash ^= ZobristHash::enpassant_hash(self.data.enpassant_bb);

        let mut is_counter_reset: bool = false; //fifty-move-rule counter

        /* special case bookkeeping */
        match source_piece {
            /* castling */
            ChessPiece(Side::White, PieceType::King) => {
                if self.data.castle_bools[0] {
                    current_hash ^= ZobristHash::castle_hash(Castling::Kingside(Side::White));
                    self.data.castle_bools[0] = false;
                }
                if self.data.castle_bools[1] {
                    current_hash ^= ZobristHash::castle_hash(Castling::Queenside(Side::White));
                    self.data.castle_bools[1] = false;
                }
            }

            ChessPiece(Side::Black, PieceType::King) => {
                if self.data.castle_bools[2] {
                    current_hash ^= ZobristHash::castle_hash(Castling::Kingside(Side::Black));
                    self.data.castle_bools[2] = false;
                }
                if self.data.castle_bools[3] {
                    current_hash ^= ZobristHash::castle_hash(Castling::Queenside(Side::Black));
                    self.data.castle_bools[3] = false;
                }
            }

            ChessPiece(Side::White, PieceType::Rook) => {
                if source == Square::W_KINGSIDE_ROOK_SQ_SOURCE {
                    if self.data.castle_bools[0] {
                        current_hash ^= ZobristHash::castle_hash(Castling::Kingside(Side::White));
                        self.data.castle_bools[0] = false;
                    }
                } else if source == Square::W_QUEENSIDE_ROOK_SQ_SOURCE {
                    if self.data.castle_bools[1] {
                        current_hash ^= ZobristHash::castle_hash(Castling::Queenside(Side::White));
                        self.data.castle_bools[1] = false
                    }
                }
            }

            ChessPiece(Side::Black, PieceType::Rook) => {
                if source == Square::B_KINGSIDE_ROOK_SQ_SOURCE {
                    if self.data.castle_bools[2] {
                        current_hash ^= ZobristHash::castle_hash(Castling::Kingside(Side::Black));
                        self.data.castle_bools[2] = false;
                    }
                } else if source == Square::B_QUEENSIDE_ROOK_SQ_SOURCE {
                    if self.data.castle_bools[3] {
                        current_hash ^= ZobristHash::castle_hash(Castling::Queenside(Side::Black));
                        self.data.castle_bools[3] = false
                    }
                }
            }

            /* enpassant and fifty-move-rule */
            ChessPiece(Side::White, PieceType::Pawn) => {
                //reset 50-move rule
                self.data.fifty_move_rule_counter = 0;
                is_counter_reset = true;
                //if move is a 2-square pawn move, update enpassant bitboard
                if self.is_pawn_move_enpassant_relevant(&source, &target) {
                    //FIXME should check if enpassant is even legal for enemy
                    enpassant_bb.set_bit(Square::nth(target.to_usize() - 8));
                }
                check_bb = check_bb.bit_or(&get_b_pawn_attack(enm_king_square).bit_and(&Bitboard::nth(target)));
            }

            ChessPiece(Side::Black, PieceType::Pawn) => {
                //reset 50-move rule
                self.data.fifty_move_rule_counter = 0;
                is_counter_reset = true;
                //if move is a 2-square pawn move, update enpassant bitboard
                if self.is_pawn_move_enpassant_relevant(&source, &target) {
                    //FIXME should check if enpassant is even legal for enemy
                    enpassant_bb.set_bit(Square::nth(target.to_usize() + 8));
                }
                check_bb = check_bb.bit_or(&get_w_pawn_attack(enm_king_square).bit_and(&Bitboard::nth(target)));
            }

            ChessPiece(_, PieceType::Knight) => check_bb = check_bb.bit_or(&get_knight_attack(enm_king_square).bit_and(&Bitboard::nth(target))),
            _ => (),
        }

        //move the piece
        self.bitboards.pop_bit(source_piece, source);
        self.bitboards.set_bit(source_piece, target);
        current_hash ^= ZobristHash::piece_hash(source, source_piece);
        current_hash ^= ZobristHash::piece_hash(target, source_piece);
        self.mailbox.set(None, source);
        self.mailbox.set(Some(source_piece), target);

        //additional book keeping
        match chess_move.move_type() {
            MoveType::Normal => {
                //dealing with captures
                if let Some(target_piece) = target_piece {
                    self.bitboards.pop_bit(target_piece, target);
                    #[cfg(feature = "piececolourboard")]
                    if source_piece.1 == target_piece.1 {
                        self.bitboards.piece[target_piece.1 as usize].set_bit(target);
                    }
                    current_hash ^= ZobristHash::piece_hash(target, target_piece);

                    //reset 50-move rule
                    self.data.fifty_move_rule_counter = 0;
                    is_counter_reset = true;

                    //if capturing enemy rook, update castling rights
                    match (target_piece, target) {
                        (ChessPiece::WR, Square::W_KINGSIDE_ROOK_SQ_SOURCE) => {
                            if self.data.castle_bools[0] {
                                current_hash ^= ZobristHash::castle_hash(Castling::Kingside(Side::White));
                                self.data.castle_bools[0] = false;
                            }
                        }
                        (ChessPiece::WR, Square::W_QUEENSIDE_ROOK_SQ_SOURCE) => {
                            if self.data.castle_bools[1] {
                                current_hash ^= ZobristHash::castle_hash(Castling::Queenside(Side::White));
                                self.data.castle_bools[1] = false;
                            }
                        }
                        (ChessPiece::BR, Square::B_KINGSIDE_ROOK_SQ_SOURCE) => {
                            if self.data.castle_bools[2] {
                                current_hash ^= ZobristHash::castle_hash(Castling::Kingside(Side::Black));
                                self.data.castle_bools[2] = false;
                            }
                        }
                        (ChessPiece::BR, Square::B_QUEENSIDE_ROOK_SQ_SOURCE) => {
                            if self.data.castle_bools[3] {
                                current_hash ^= ZobristHash::castle_hash(Castling::Queenside(Side::Black));
                                self.data.castle_bools[3] = false;
                            }
                        }
                        _ => (),
                    }
                }
            }

            MoveType::Castle(castling) => {
                let (piece, rook_square_source, rook_square_target) = match castling {
                    Castling::Kingside(Side::White) => (ChessPiece::WR, Square::W_KINGSIDE_ROOK_SQ_SOURCE, Square::W_KINGSIDE_ROOK_SQ_TARGET),
                    Castling::Queenside(Side::White) => (ChessPiece::WR, Square::W_QUEENSIDE_ROOK_SQ_SOURCE, Square::W_QUEENSIDE_ROOK_SQ_TARGET),
                    Castling::Kingside(Side::Black) => (ChessPiece::BR, Square::B_KINGSIDE_ROOK_SQ_SOURCE, Square::B_KINGSIDE_ROOK_SQ_TARGET),
                    Castling::Queenside(Side::Black) => (ChessPiece::BR, Square::B_QUEENSIDE_ROOK_SQ_SOURCE, Square::B_QUEENSIDE_ROOK_SQ_TARGET),
                };
                debug_assert!(self.bitboards.piece_bitboard(piece).nth_is_not_zero(rook_square_source));
                self.bitboards.pop_bit(piece, rook_square_source);
                self.bitboards.set_bit(piece, rook_square_target);
                self.mailbox.set(None, rook_square_source);
                self.mailbox.set(Some(piece), rook_square_target);

                //update hash
                current_hash ^= ZobristHash::piece_hash(rook_square_source, piece);
                current_hash ^= ZobristHash::piece_hash(rook_square_target, piece);
            }

            MoveType::EnPassant => {
                let enemy_pawn_square: Square;
                let enemy_piece: ChessPiece = ChessPiece(side.update(), PieceType::Pawn);
                match self.data.side_to_move {
                    Side::White => {
                        enemy_pawn_square = Square::nth(target.to_usize() - 8);
                    }
                    Side::Black => {
                        enemy_pawn_square = Square::nth(target.to_usize() + 8);
                    }
                }

                debug_assert!(self.bitboards.piece_bitboard(enemy_piece).nth_is_not_zero(enemy_pawn_square));
                debug_assert!(
                    self.mailbox.square_index(enemy_pawn_square) == Some(ChessPiece::WP)
                        || self.mailbox.square_index(enemy_pawn_square) == Some(ChessPiece::BP)
                );

                self.bitboards.pop_bit(enemy_piece, enemy_pawn_square);
                current_hash ^= ZobristHash::piece_hash(enemy_pawn_square, enemy_piece);
                self.mailbox.set(None, enemy_pawn_square);
            }

            MoveType::Promotion(piece_type) => {
                if piece_type == PieceType::Knight {
                    check_bb = check_bb.bit_or(&get_knight_attack(enm_king_square).bit_and(&Bitboard::nth(target)));
                }

                let promoted_piece = ChessPiece(self.data.side_to_move, piece_type);

                //dealing with captures
                if let Some(target_piece) = target_piece {
                    self.bitboards.pop_bit(target_piece, target);
                    #[cfg(feature = "piececolourboard")]
                    if source_piece.1 == target_piece.1 {
                        self.bitboards.piece[target_piece.1 as usize].set_bit(target);
                    }
                    current_hash ^= ZobristHash::piece_hash(target, target_piece);

                    //reset 50-move rule
                    self.data.fifty_move_rule_counter = 0;
                    is_counter_reset = true;

                    //if capturing enemy rook, update castling rights
                    match (target_piece, target) {
                        (ChessPiece::WR, Square::W_KINGSIDE_ROOK_SQ_SOURCE) => {
                            if self.data.castle_bools[0] {
                                current_hash ^= ZobristHash::castle_hash(Castling::Kingside(Side::White));
                            }
                            self.data.castle_bools[0] = false;
                        }
                        (ChessPiece::WR, Square::W_QUEENSIDE_ROOK_SQ_SOURCE) => {
                            if self.data.castle_bools[1] {
                                current_hash ^= ZobristHash::castle_hash(Castling::Queenside(Side::White));
                            }
                            self.data.castle_bools[1] = false;
                        }
                        (ChessPiece::BR, Square::B_KINGSIDE_ROOK_SQ_SOURCE) => {
                            if self.data.castle_bools[2] {
                                current_hash ^= ZobristHash::castle_hash(Castling::Kingside(Side::Black));
                            }
                            self.data.castle_bools[2] = false;
                        }
                        (ChessPiece::BR, Square::B_QUEENSIDE_ROOK_SQ_SOURCE) => {
                            if self.data.castle_bools[3] {
                                current_hash ^= ZobristHash::castle_hash(Castling::Queenside(Side::Black));
                            }
                            self.data.castle_bools[3] = false;
                        }
                        _ => (),
                    }
                }

                //remove the pawn piece
                self.bitboards.pop_bit(source_piece, target);
                current_hash ^= ZobristHash::piece_hash(target, source_piece);

                //add the promoted piece
                self.bitboards.set_bit(promoted_piece, target);
                current_hash ^= ZobristHash::piece_hash(target, promoted_piece);
                self.mailbox.set(Some(promoted_piece), target);
            }
        }

        //cozy-chess tech
        //note that previously check_bb contains all checking knight pieces

        // pieces: white pawn, white knight, white bishop, white rook, white queen, white king,
        //         black pawn, black knight, black bishop, black rook, black queen, black king,
        let mut check_mask: Bitboard = check_bb;
        //note that attackers can only ever be: a rook, a bishop, or a queen
        let bishops_or_queens: Bitboard;
        let rooks_or_queens: Bitboard;
        match self.side() {
            Side::White => {
                bishops_or_queens = self.bitboards.piece_bitboard(ChessPiece::WQ).bit_or(&self.bitboards.piece_bitboard(ChessPiece::WB));
                rooks_or_queens = self.bitboards.piece_bitboard(ChessPiece::WQ).bit_or(&self.bitboards.piece_bitboard(ChessPiece::WR));
            }
            Side::Black => {
                bishops_or_queens = self.bitboards.piece_bitboard(ChessPiece::BQ).bit_or(&self.bitboards.piece_bitboard(ChessPiece::BB));
                rooks_or_queens = self.bitboards.piece_bitboard(ChessPiece::BQ).bit_or(&self.bitboards.piece_bitboard(ChessPiece::BR));
            }
        }
        let bishop_ray_hits = get_bishop_ray(enm_king_square).bit_and(&bishops_or_queens);
        let rook_ray_hits = get_rook_ray(enm_king_square).bit_and(&rooks_or_queens);
        let mut attackers: Bitboard = bishop_ray_hits.bit_or(&rook_ray_hits);

        //note that attackers can only ever be: a rook, a bishop, or a queen
        while attackers.is_not_zero() {
            let attacker_square: Square = attackers.lsb_square().unwrap();
            let attacker_bb: Bitboard = attackers.lsb_bitboard();
            let ray: Bitboard = rays(attacker_square, enm_king_square);
            let pinned_pieces: Bitboard = ray.bit_and(&self.bitboards.blockers());
            match pinned_pieces.count_ones() {
                0 => {
                    check_bb = check_bb.bit_or(&attacker_bb);
                    check_mask = check_mask.bit_or(&attacker_bb.bit_or(&ray));
                }
                1 => {
                    pinned_bb = pinned_bb.bit_or(&pinned_pieces);
                    pinner_bb = pinner_bb.bit_or(&attacker_bb);
                }
                _ => (),
            }
            attackers.pop_lsb();
        }

        if self.data.side_to_move == Side::Black {
            self.data.full_move_counter += 1;
        }

        self.data.side_to_move = self.data.side_to_move.update();
        current_hash ^= ZobristHash::side_hash();
        if is_counter_reset == false {
            self.data.fifty_move_rule_counter += 1;
        }

        self.data.enpassant_bb = enpassant_bb;
        current_hash ^= ZobristHash::enpassant_hash(enpassant_bb);

        self.data.zobrist_hash = current_hash;

        //self.compute_check_bb();
        self.data.check_bb = check_bb;
        self.data.check_mask = check_mask;

        //self.compute_pin_data();
        self.data.pinner_bb = pinner_bb;
        self.data.pinned_bb = pinned_bb;
    }

    #[inline(always)]
    fn is_pawn_move_enpassant_relevant(&self, source: &Square, target: &Square) -> bool {
        match self.side() {
            Side::White => {
                (source.to_usize() + 16 == target.to_usize())
                    && ((matches!(self.mailbox.square_index(target.right()), Some(ChessPiece::BP)) && (source.to_col_usize() != 7))
                        || matches!(self.mailbox.square_index(target.left()), Some(ChessPiece::BP)) && (source.to_col_usize() != 0))
            }
            Side::Black => {
                (source.to_usize() == target.to_usize() + 16)
                    && (matches!(self.mailbox.square_index(target.right()), Some(ChessPiece::WP)) && (source.to_col_usize() != 7)
                        || matches!(self.mailbox.square_index(target.left()), Some(ChessPiece::WP)) && (source.to_col_usize() != 0))
            }
        }
    }
}
