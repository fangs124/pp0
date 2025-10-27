use crate::{
    Bitboard, ChessBoard, ChessPiece, PieceType, Side,
    bitboard::attack::{get_b_pawn_attack, get_bishop_attack, get_knight_attack, get_queen_attack, get_rook_attack, get_w_pawn_attack, rays},
    chessboard::{ChessData, PieceBitboard, chess_piece, mailbox::Mailbox, zobrist::ZobristHash},
    square::Square,
};

impl ChessBoard {
    pub fn from_fen(input: &str) -> ChessBoard {
        assert!(input.is_ascii());
        let mut input = input.split_ascii_whitespace();

        //let mut piece_board: PieceBoard = PieceBoard::EMPTY_BOARD;
        let mut bitboards: PieceBitboard = PieceBitboard::EMPTY_BOARD;
        let mut mailbox: Mailbox = Mailbox::EMPTY_MAILBOX;
        let mut castle_bools = [false, false, false, false];

        // example fen: rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1

        // parse piece placement data
        let mut square: usize = 0;
        for row in input.next().expect("from_fen error: missing pieces placement token").rsplit('/').collect::<Vec<&str>>() {
            for c in row.chars() {
                match c {
                    //TODO find a better way to do this?
                    c @ ('K' | 'Q' | 'N' | 'B' | 'R' | 'P' | 'k' | 'q' | 'n' | 'b' | 'r' | 'p') => {
                        bitboards.set_bit(chess_piece(c), Square::nth(square));
                        mailbox.set(Some(c.try_into().expect(&format!("from_fen error: invalid char {c}"))), Square::nth(square));
                    }

                    '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' => {
                        square += (c.to_digit(10).unwrap() as usize) - 1;
                    }

                    _ => panic!("from_fen error: invalid char {c}"),
                }
                square += 1;
            }
        }

        // parse active colour
        let side_to_move = match input.next().expect("from_fen error: missing active side token") {
            "w" => Side::White,
            "b" => Side::Black,
            _ => panic!("from_fen error: invalid active side token"),
        };

        // parse castling information
        for s in input.next().expect("from_fen error: missing castling rights token").chars() {
            match s {
                '-' => (),
                'K' => castle_bools[0] = true,
                'Q' => castle_bools[1] = true,
                'k' => castle_bools[2] = true,
                'q' => castle_bools[3] = true,
                _ => panic!("from_fen error: invalid castling rights token"),
            }
        }

        let mut enpassant_bb: Bitboard = Bitboard::ZERO;
        //parse en passant information
        let en_passant_token = input.next().expect("from_fen error: missing en passant token");
        if en_passant_token != "-" {
            assert!(en_passant_token.len() == 2, "from_fen error: incorrect en passant token length");
            enpassant_bb = Bitboard::nth(Square::parse_str(en_passant_token));
        }

        //parse fifty-move-rule counter
        let fifty_move_rule_counter = input.next().map_or(0, |x| x.parse::<u16>().expect("from_fen error: invalid fifty-move-rule token"));

        //parse fullmove number
        let full_move_counter = input.next().map_or(0, |x| x.parse::<u16>().expect("from_fen error: invalid move-counter token"));

        //check bitboard
        let blockers: Bitboard = bitboards.blockers();
        let enemy_side: Side = side_to_move.update();
        let king_square: Square = bitboards.piece_bitboard(ChessPiece(side_to_move, PieceType::King)).lsb_square().unwrap();
        let check_bb: Bitboard = {
            let queen_bb: Bitboard = bitboards.piece_bitboard(ChessPiece(enemy_side, PieceType::Queen)).bit_and(&get_queen_attack(king_square, blockers));
            let knight_bb: Bitboard = bitboards.piece_bitboard(ChessPiece(enemy_side, PieceType::Knight)).bit_and(&get_knight_attack(king_square));
            let bishop_bb: Bitboard = bitboards.piece_bitboard(ChessPiece(enemy_side, PieceType::Bishop)).bit_and(&get_bishop_attack(king_square, blockers));
            let rook_bb: Bitboard = bitboards.piece_bitboard(ChessPiece(enemy_side, PieceType::Rook)).bit_and(&get_rook_attack(king_square, blockers));
            let pawn_bb: Bitboard = match side_to_move {
                Side::White => bitboards.piece_bitboard(ChessPiece(enemy_side, PieceType::Pawn)).bit_and(&get_w_pawn_attack(king_square)),
                Side::Black => bitboards.piece_bitboard(ChessPiece(enemy_side, PieceType::Pawn)).bit_and(&get_b_pawn_attack(king_square)),
            };
            queen_bb.bit_or(&knight_bb.bit_or(&bishop_bb.bit_or(&rook_bb.bit_or(&pawn_bb))))
        };

        let mut pinner_bb: Bitboard = Bitboard::ZERO;
        let mut pinned_bb: Bitboard = Bitboard::ZERO;

        let enemy_knight_piece: ChessPiece = match side_to_move {
            Side::White => ChessPiece(Side::Black, PieceType::Knight),
            Side::Black => ChessPiece(Side::White, PieceType::Knight),
        };

        let mut non_knight_check_bb: Bitboard = check_bb.bit_and(&bitboards.piece_bitboard(enemy_knight_piece).bit_not());
        let mut check_mask: Bitboard = check_bb.clone();
        while non_knight_check_bb.is_not_zero() {
            let checker_square = non_knight_check_bb.lsb_square().unwrap();
            check_mask = check_mask.bit_or(&rays(checker_square, king_square));
            non_knight_check_bb.pop_lsb();
        }

        let friends: Bitboard;
        let enemies: Bitboard;
        let diagonal_enemies: Bitboard;
        let lateral_enemies: Bitboard;

        match side_to_move {
            Side::White => {
                friends = bitboards.white_blockers();
                enemies = bitboards.black_blockers();
                diagonal_enemies = bitboards.piece_bitboard(ChessPiece::BQ).bit_or(&bitboards.piece_bitboard(ChessPiece::BB));
                lateral_enemies = bitboards.piece_bitboard(ChessPiece::BQ).bit_or(&bitboards.piece_bitboard(ChessPiece::BR));
            }
            Side::Black => {
                friends = bitboards.black_blockers();
                enemies = bitboards.white_blockers();
                diagonal_enemies = bitboards.piece_bitboard(ChessPiece::WQ).bit_or(&bitboards.piece_bitboard(ChessPiece::WB));
                lateral_enemies = bitboards.piece_bitboard(ChessPiece::WQ).bit_or(&bitboards.piece_bitboard(ChessPiece::WR));
            }
        }

        assert!(bitboards.piece_bitboard(ChessPiece(side_to_move, PieceType::King)).count_ones() == 1);
        let king_square = bitboards.piece_bitboard(ChessPiece(side_to_move, PieceType::King)).lsb_square().unwrap();
        let mut possible_pinners: Bitboard = (get_bishop_attack(king_square, diagonal_enemies).bit_and(&diagonal_enemies))
            .bit_or(&get_rook_attack(king_square, lateral_enemies).bit_and(&lateral_enemies));
        while possible_pinners.is_not_zero() {
            let possible_pinner = possible_pinners.lsb_square().unwrap();
            let pinner_piece: ChessPiece = mailbox.square_index(possible_pinner).unwrap();
            let attack_mask = match pinner_piece {
                ChessPiece(_, PieceType::Bishop) => get_bishop_attack(possible_pinner, enemies),
                ChessPiece(_, PieceType::Rook) => get_rook_attack(possible_pinner, enemies),
                ChessPiece(_, PieceType::Queen) => get_queen_attack(possible_pinner, enemies),
                _ => panic!(),
            };

            let relevant_mask: Bitboard = rays(king_square, possible_pinner).bit_and(&attack_mask);
            let enemy_blockers: Bitboard = relevant_mask.bit_and(&enemies);
            let possible_pinned: Bitboard = relevant_mask.bit_and(&friends);

            //NOTE: a piece is only pinned if and only if it is the only piece between the pinner and the king.
            //      enemy can also block the line of sight.
            if possible_pinned.count_ones() == 1 && enemy_blockers.count_ones() == 0 {
                pinner_bb = pinner_bb.bit_or(&possible_pinners.lsb_bitboard());
                pinned_bb = pinned_bb.bit_or(&possible_pinned);
            }

            possible_pinners.pop_lsb();
        }

        let zobrist_hash: ZobristHash = ZobristHash::compute_hash(side_to_move, &mailbox, castle_bools, enpassant_bb);

        let data: ChessData = ChessData {
            castle_bools,
            enpassant_bb,
            check_bb,
            check_mask,
            pinned_bb,
            pinner_bb,
            side_to_move,
            full_move_counter,
            fifty_move_rule_counter,
            zobrist_hash,
        };
        ChessBoard { bitboards, mailbox, data }
    }
}
