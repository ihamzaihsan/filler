#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<char>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Piece {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<char>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    One,
    Two,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Coord {
    pub x: i32,
    pub y: i32,
}

impl Player {
    pub fn from_exec_line(line: &str) -> Option<Self> {
        if line.contains("p1") {
            Some(Self::One)
        } else if line.contains("p2") {
            Some(Self::Two)
        } else {
            None
        }
    }
}

impl Piece {
    pub fn filled_offsets(&self) -> Vec<(i32, i32)> {
        let mut offsets = Vec::new();

        for y in 0..self.height {
            for x in 0..self.width {
                if self.cells[y][x] != '.' {
                    offsets.push((x as i32, y as i32));
                }
            }
        }

        offsets
    }
}

pub fn parse_dimensions(line: &str, label: &str) -> Option<(usize, usize)> {
    let normalized = line.trim_end_matches(':');
    let mut parts = normalized.split_whitespace();

    if parts.next()? != label {
        return None;
    }

    let width = parts.next()?.parse().ok()?;
    let height = parts.next()?.parse().ok()?;

    Some((width, height))
}

pub fn parse_board(header: &str, lines: &[String]) -> Option<Board> {
    let (width, height) = parse_dimensions(header, "Anfield")?;
    if lines.len() != height {
        return None;
    }

    let mut cells = Vec::with_capacity(height);
    for line in lines {
        let row = line.split_whitespace().last()?;
        let row_cells: Vec<char> = row.chars().collect();
        if row_cells.len() != width {
            return None;
        }
        cells.push(row_cells);
    }

    Some(Board {
        width,
        height,
        cells,
    })
}

pub fn parse_piece(header: &str, lines: &[String]) -> Option<Piece> {
    let (width, height) = parse_dimensions(header, "Piece")?;
    if lines.len() != height {
        return None;
    }

    let mut cells = Vec::with_capacity(height);
    for line in lines {
        let row_cells: Vec<char> = line.trim().chars().collect();
        if row_cells.len() != width {
            return None;
        }
        cells.push(row_cells);
    }

    Some(Piece {
        width,
        height,
        cells,
    })
}

pub fn parse_turn(input: &str) -> Option<(Board, Piece)> {
    let mut lines = input.lines();

    let board_header = lines.find(|line| line.starts_with("Anfield "))?;
    let (_, height) = parse_dimensions(board_header, "Anfield")?;
    lines.next()?;

    let board_lines: Vec<String> = lines.by_ref().take(height).map(String::from).collect();
    let piece_header = lines.find(|line| line.starts_with("Piece "))?;
    let (_, piece_height) = parse_dimensions(piece_header, "Piece")?;
    let piece_lines: Vec<String> = lines.take(piece_height).map(String::from).collect();

    Some((
        parse_board(board_header, &board_lines)?,
        parse_piece(piece_header, &piece_lines)?,
    ))
}

pub fn format_coord(coord: Coord) -> String {
    format!("{} {}\n", coord.x, coord.y)
}
