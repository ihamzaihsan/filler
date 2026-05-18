use filler::{
    find_best_placement, format_coord, is_valid_placement, parse_turn, Board, Coord, Piece, Player,
};

#[test]
fn parses_anfield_and_piece_from_engine_input() {
    let input = "\
$$$ exec p1 : [solution/target/release/filler]
Anfield 20 15:
    01234567890123456789
000 ....................
001 ....................
002 .........@..........
003 ....................
004 ....................
005 ....................
006 ....................
007 ....................
008 ....................
009 ....................
010 ....................
011 ....................
012 .........$..........
013 ....................
014 ....................
Piece 4 1:
.OO.
";

    let (board, piece) = parse_turn(input).expect("turn should parse");

    assert_eq!(board.width, 20);
    assert_eq!(board.height, 15);
    assert_eq!(board.cells[2][9], '@');
    assert_eq!(board.cells[12][9], '$');
    assert_eq!(piece.width, 4);
    assert_eq!(piece.height, 1);
    assert_eq!(piece.cells[0], vec!['.', 'O', 'O', '.']);
}

#[test]
fn validates_exactly_one_own_overlap_and_no_opponent_overlap() {
    let board = Board {
        width: 5,
        height: 5,
        cells: vec![
            ".....".chars().collect(),
            "..@..".chars().collect(),
            "...$.".chars().collect(),
            ".....".chars().collect(),
            ".....".chars().collect(),
        ],
    };
    let piece = Piece {
        width: 2,
        height: 1,
        cells: vec!["OO".chars().collect()],
    };

    assert!(is_valid_placement(
        &board,
        &piece,
        Player::One,
        Coord { x: 1, y: 1 }
    ));
    assert!(!is_valid_placement(
        &board,
        &piece,
        Player::One,
        Coord { x: 2, y: 2 }
    ));
}

#[test]
fn rejects_piece_that_extends_beyond_anfield_boundary() {
    let board = Board {
        width: 3,
        height: 3,
        cells: vec![
            "@..".chars().collect(),
            "...".chars().collect(),
            "...".chars().collect(),
        ],
    };
    let piece = Piece {
        width: 2,
        height: 2,
        cells: vec!["OO".chars().collect(), ".O".chars().collect()],
    };

    assert!(!is_valid_placement(
        &board,
        &piece,
        Player::One,
        Coord { x: 2, y: 0 }
    ));
    assert!(!is_valid_placement(
        &board,
        &piece,
        Player::One,
        Coord { x: -1, y: 0 }
    ));
}

#[test]
fn formats_coordinate_exactly_as_required() {
    assert_eq!(format_coord(Coord { x: 7, y: 2 }), "7 2\n");
}

#[test]
fn chooses_the_readme_example_move_when_it_is_best() {
    let input = "\
Anfield 20 15:
    01234567890123456789
000 ....................
001 ....................
002 .........@..........
003 ....................
004 ....................
005 ....................
006 ....................
007 ....................
008 ....................
009 ....................
010 ....................
011 ....................
012 .........$..........
013 ....................
014 ....................
Piece 4 1:
.OO.
";

    let (board, piece) = parse_turn(input).expect("turn should parse");
    let coord = find_best_placement(&board, &piece, Player::One).expect("move should exist");

    assert_eq!(coord, Coord { x: 7, y: 2 });
}
