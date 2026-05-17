use std::cmp::Ordering;

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

    fn own_cells(self) -> [char; 2] {
        match self {
            Self::One => ['a', '@'],
            Self::Two => ['s', '$'],
        }
    }

    fn opponent_cells(self) -> [char; 2] {
        match self {
            Self::One => ['s', '$'],
            Self::Two => ['a', '@'],
        }
    }

    fn is_own(self, value: char) -> bool {
        self.own_cells().contains(&value)
    }

    fn is_opponent(self, value: char) -> bool {
        self.opponent_cells().contains(&value)
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

pub fn is_valid_placement(board: &Board, piece: &Piece, player: Player, coord: Coord) -> bool {
    let mut own_overlaps = 0;

    for (piece_x, piece_y) in piece.filled_offsets() {
        let board_x = coord.x + piece_x;
        let board_y = coord.y + piece_y;

        if board_x < 0
            || board_y < 0
            || board_x >= board.width as i32
            || board_y >= board.height as i32
        {
            return false;
        }

        let board_cell = board.cells[board_y as usize][board_x as usize];
        if player.is_opponent(board_cell) {
            return false;
        }
        if player.is_own(board_cell) {
            own_overlaps += 1;
            if own_overlaps > 1 {
                return false;
            }
        }
    }

    own_overlaps == 1
}

pub fn find_best_placement(board: &Board, piece: &Piece, player: Player) -> Option<Coord> {
    let offsets = piece.filled_offsets();
    if offsets.is_empty() {
        return None;
    }

    let opponents = collect_cells(board, |cell| player.is_opponent(cell));
    let own = collect_cells(board, |cell| player.is_own(cell));

    let min_x = -offsets.iter().map(|(x, _)| *x).max().unwrap_or(0);
    let min_y = -offsets.iter().map(|(_, y)| *y).max().unwrap_or(0);
    let max_x = board.width as i32 - offsets.iter().map(|(x, _)| *x).min().unwrap_or(0) - 1;
    let max_y = board.height as i32 - offsets.iter().map(|(_, y)| *y).min().unwrap_or(0) - 1;

    let mut best: Option<(Coord, PlacementScore)> = None;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let coord = Coord { x, y };
            if !is_valid_placement(board, piece, player, coord) {
                continue;
            }

            let score = score_placement(coord, &offsets, &opponents, &own);
            match best {
                None => best = Some((coord, score)),
                Some((_, current)) if score < current => best = Some((coord, score)),
                _ => {}
            }
        }
    }

    best.map(|(coord, _)| coord)
}

pub fn format_coord(coord: Coord) -> String {
    format!("{} {}\n", coord.x, coord.y)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PlacementScore {
    opponent_distance: i32,
    own_distance: i32,
    y: i32,
    x: i32,
}

impl Ord for PlacementScore {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.opponent_distance, self.own_distance, self.y, self.x).cmp(&(
            other.opponent_distance,
            other.own_distance,
            other.y,
            other.x,
        ))
    }
}

impl PartialOrd for PlacementScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn score_placement(
    coord: Coord,
    offsets: &[(i32, i32)],
    opponents: &[Coord],
    own: &[Coord],
) -> PlacementScore {
    let placed_cells: Vec<Coord> = offsets
        .iter()
        .map(|(x, y)| Coord {
            x: coord.x + x,
            y: coord.y + y,
        })
        .collect();

    PlacementScore {
        opponent_distance: min_distance(&placed_cells, opponents).unwrap_or(0),
        own_distance: -min_distance(&placed_cells, own).unwrap_or(0),
        y: coord.y,
        x: coord.x,
    }
}

fn collect_cells(board: &Board, predicate: impl Fn(char) -> bool) -> Vec<Coord> {
    let mut coords = Vec::new();

    for y in 0..board.height {
        for x in 0..board.width {
            if predicate(board.cells[y][x]) {
                coords.push(Coord {
                    x: x as i32,
                    y: y as i32,
                });
            }
        }
    }

    coords
}

fn min_distance(from: &[Coord], to: &[Coord]) -> Option<i32> {
    from.iter()
        .flat_map(|a| {
            to.iter()
                .map(move |b| (a.x - b.x).abs() + (a.y - b.y).abs())
        })
        .min()
}
