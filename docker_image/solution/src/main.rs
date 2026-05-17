use filler::{
    find_best_placement, format_coord, parse_board, parse_dimensions, parse_piece, Player,
};
use std::io::{self, BufRead, Write};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        print!("0 0\n");
    }
}

fn run() -> io::Result<()> {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let mut stdout = io::stdout();

    let exec_line = match next_line(&mut lines)? {
        Some(line) => line,
        None => return Ok(()),
    };
    let player = Player::from_exec_line(&exec_line).unwrap_or(Player::One);

    while let Some(board_header) = read_until_prefix(&mut lines, "Anfield ")? {
        let (_, board_height) = match parse_dimensions(&board_header, "Anfield") {
            Some(dimensions) => dimensions,
            None => break,
        };

        let _column_header = next_line(&mut lines)?;
        let mut board_lines = Vec::with_capacity(board_height);
        for _ in 0..board_height {
            if let Some(line) = next_line(&mut lines)? {
                board_lines.push(line);
            }
        }

        let piece_header = match read_until_prefix(&mut lines, "Piece ")? {
            Some(line) => line,
            None => break,
        };
        let (_, piece_height) = match parse_dimensions(&piece_header, "Piece") {
            Some(dimensions) => dimensions,
            None => break,
        };

        let mut piece_lines = Vec::with_capacity(piece_height);
        for _ in 0..piece_height {
            if let Some(line) = next_line(&mut lines)? {
                piece_lines.push(line);
            }
        }

        let coord = parse_board(&board_header, &board_lines)
            .and_then(|board| parse_piece(&piece_header, &piece_lines).map(|piece| (board, piece)))
            .and_then(|(board, piece)| find_best_placement(&board, &piece, player))
            .unwrap_or(filler::Coord { x: 0, y: 0 });

        stdout.write_all(format_coord(coord).as_bytes())?;
        stdout.flush()?;
    }

    Ok(())
}

fn read_until_prefix(
    lines: &mut impl Iterator<Item = io::Result<String>>,
    prefix: &str,
) -> io::Result<Option<String>> {
    for line in lines {
        let line = line?;
        if line.starts_with(prefix) {
            return Ok(Some(line));
        }
    }

    Ok(None)
}

fn next_line(lines: &mut impl Iterator<Item = io::Result<String>>) -> io::Result<Option<String>> {
    lines.next().transpose()
}
