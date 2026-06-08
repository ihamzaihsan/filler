//! Streaming protocol parsing, legal move validation, and deterministic territory strategy.

use std::collections::VecDeque;
use std::io::{self, BufRead, Cursor};

// Bound allocations from protocol dimensions; supplied maps are at most 9,900 cells.
const MAX_CELLS: usize = 1_000_000;
const FULL_EVALUATION_CELLS: usize = 2_000;
const CAUTIOUS_EXPANSION_CELLS: usize = 512;
const MAX_TERRITORY_CANDIDATES: usize = 64;
const MAX_REPLY_ROOTS: usize = 8;
const MAX_REPLY_CANDIDATES: usize = 12;
// Scenarios, not predictions: the next random shape is not known.
const REPLY_SHAPES: &[&[(i32, i32)]] = &[
    &[(0, 0), (1, 0), (0, 1), (1, 1)],
    &[(0, 0), (1, 0), (0, 1)],
    &[(0, 0), (1, 1)],
];
type MoveRank = (i64, i64, i32, i32);
type RankedMove = (MoveRank, Coord);

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

/// Engine coordinates: x is the column, y is the row, both zero-based.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Coord {
    pub x: i32,
    pub y: i32,
}

impl Player {
    pub fn from_exec_line(line: &str) -> Option<Self> {
        let mut parts = line.split_whitespace();
        if parts.next()? != "$$$" || parts.next()? != "exec" {
            return None;
        }
        let player = match parts.next()? {
            "p1" => Self::One,
            "p2" => Self::Two,
            _ => return None,
        };
        if parts.next()? != ":" {
            return None;
        }
        Some(player)
    }

    fn is_own(self, value: char) -> bool {
        match self {
            Self::One => matches!(value, 'a' | '@'),
            Self::Two => matches!(value, 's' | '$'),
        }
    }

    fn is_opponent(self, value: char) -> bool {
        match self {
            Self::One => matches!(value, 's' | '$'),
            Self::Two => matches!(value, 'a' | '@'),
        }
    }

    fn opponent(self) -> Self {
        match self {
            Self::One => Self::Two,
            Self::Two => Self::One,
        }
    }
}

impl Piece {
    pub fn filled_offsets(&self) -> Vec<(i32, i32)> {
        self.cells
            .iter()
            .enumerate()
            .flat_map(|(y, row)| {
                row.iter()
                    .enumerate()
                    .filter_map(move |(x, cell)| (*cell != '.').then_some((x as i32, y as i32)))
            })
            .collect()
    }
}

pub fn parse_dimensions(line: &str, label: &str) -> Option<(usize, usize)> {
    let mut parts = line.trim().strip_suffix(':')?.split_whitespace();
    if parts.next()? != label {
        return None;
    }
    let width: usize = parts.next()?.parse().ok()?;
    let height: usize = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !valid_dimensions(width, height) {
        return None;
    }
    Some((width, height))
}

fn valid_dimensions(width: usize, height: usize) -> bool {
    width > 0 && height > 0 && width.checked_mul(height).is_some_and(|n| n <= MAX_CELLS)
}

pub fn parse_board(header: &str, lines: &[String]) -> Option<Board> {
    let (width, height) = parse_dimensions(header, "Anfield")?;
    if lines.len() != height {
        return None;
    }
    let mut cells = Vec::with_capacity(height);
    for (index, line) in lines.iter().enumerate() {
        let mut parts = line.split_whitespace();
        if parts.next()?.parse::<usize>().ok()? != index {
            return None;
        }
        let row: Vec<char> = parts.next()?.chars().collect();
        if parts.next().is_some()
            || row.len() != width
            || row
                .iter()
                .any(|c| !matches!(c, '.' | '@' | '$' | 'a' | 's'))
        {
            return None;
        }
        cells.push(row);
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
        let row: Vec<char> = line.trim().chars().collect();
        if row.len() != width || row.iter().any(|c| !matches!(c, '.' | 'O' | '#')) {
            return None;
        }
        cells.push(row);
    }
    Some(Piece {
        width,
        height,
        cells,
    })
}

/// Reads one board and piece at a time without waiting for stdin to close.
pub struct TurnReader<R> {
    input: R,
}

impl<R: BufRead> TurnReader<R> {
    pub fn new(input: R) -> Self {
        Self { input }
    }

    pub fn read_turn(&mut self) -> io::Result<Option<(Board, Piece)>> {
        let board_header = loop {
            let mut line = String::new();
            if self.input.read_line(&mut line)? == 0 {
                return Ok(None);
            }
            if line.starts_with("Anfield ") {
                break line;
            }
        };
        let (_, height) = parse_dimensions(&board_header, "Anfield")
            .ok_or_else(|| invalid_data("invalid Anfield dimensions"))?;
        self.required_line()?; // Column ruler, followed by exactly height indexed rows.
        let board_lines = self.read_rows(height)?;
        let board = parse_board(&board_header, &board_lines)
            .ok_or_else(|| invalid_data("invalid Anfield rows"))?;
        let piece_header = self.required_line()?;
        let (_, height) = parse_dimensions(&piece_header, "Piece")
            .ok_or_else(|| invalid_data("invalid Piece dimensions"))?;
        let piece_lines = self.read_rows(height)?;
        let piece = parse_piece(&piece_header, &piece_lines)
            .ok_or_else(|| invalid_data("invalid Piece rows"))?;
        Ok(Some((board, piece)))
    }

    fn required_line(&mut self) -> io::Result<String> {
        let mut line = String::new();
        if self.input.read_line(&mut line)? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "incomplete turn",
            ));
        }
        Ok(line.trim_end_matches(['\r', '\n']).to_owned())
    }

    fn read_rows(&mut self, height: usize) -> io::Result<Vec<String>> {
        (0..height).map(|_| self.required_line()).collect()
    }
}

fn invalid_data(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub fn parse_turn(input: &str) -> Option<(Board, Piece)> {
    TurnReader::new(Cursor::new(input))
        .read_turn()
        .ok()
        .flatten()
}

fn valid_grid(width: usize, height: usize, cells: &[Vec<char>]) -> bool {
    valid_dimensions(width, height)
        && cells.len() == height
        && cells.iter().all(|row| row.len() == width)
}

pub fn is_valid_placement(board: &Board, piece: &Piece, player: Player, coord: Coord) -> bool {
    valid_grid(board.width, board.height, &board.cells)
        && valid_grid(piece.width, piece.height, &piece.cells)
        && valid_offsets(board, &piece.filled_offsets(), player, coord)
}

fn valid_offsets(board: &Board, offsets: &[(i32, i32)], player: Player, coord: Coord) -> bool {
    let mut own_overlaps = 0;
    for &(piece_x, piece_y) in offsets {
        // i64 also handles extreme coordinates supplied to the public validator.
        let x = i64::from(coord.x) + i64::from(piece_x);
        let y = i64::from(coord.y) + i64::from(piece_y);
        if x < 0 || y < 0 || x >= board.width as i64 || y >= board.height as i64 {
            return false;
        }
        let cell = board.cells[y as usize][x as usize];
        if player.is_opponent(cell) {
            return false;
        }
        if player.is_own(cell) {
            own_overlaps += 1;
            if own_overlaps > 1 {
                return false;
            }
        }
    }
    own_overlaps == 1
}

/// Scores reachable territory, with bounded reply scenarios on compact boards.
/// Boards above 2,000 cells evaluate the 64 best geometric candidates.
pub fn find_best_placement(board: &Board, piece: &Piece, player: Player) -> Option<Coord> {
    if !valid_grid(board.width, board.height, &board.cells)
        || !valid_grid(piece.width, piece.height, &piece.cells)
    {
        return None;
    }
    let offsets = piece.filled_offsets();
    let mut candidates = ranked_candidates(board, &offsets, player);
    if board.width * board.height > FULL_EVALUATION_CELLS {
        candidates.truncate(MAX_TERRITORY_CANDIDATES);
    }
    let mut evaluated: Vec<_> = candidates
        .into_iter()
        .map(|(tie, coord)| {
            let next = placed_board(board, &offsets, player, coord);
            (territory_potential(&next, player), tie, coord)
        })
        .collect();
    evaluated.sort_unstable_by_key(|(value, tie, _)| (-*value, *tie));
    if board.width * board.height > CAUTIOUS_EXPANSION_CELLS {
        return evaluated.first().map(|(_, _, coord)| *coord);
    }
    evaluated.truncate(MAX_REPLY_ROOTS);
    let mut best = None;
    for (value, tie, coord) in evaluated {
        let next = placed_board(board, &offsets, player, coord);
        let mut reply_total = 0;
        for shape in std::iter::once(offsets.as_slice()).chain(REPLY_SHAPES.iter().copied()) {
            let mut reply_value = value;
            for (_, reply) in ranked_candidates(&next, shape, player.opponent())
                .into_iter()
                .take(MAX_REPLY_CANDIDATES)
            {
                let after = placed_board(&next, shape, player.opponent(), reply);
                reply_value = reply_value.min(territory_potential(&after, player));
            }
            reply_total += reply_value;
        }
        // Keep the immediate estimate dominant: hypothetical shapes may never arrive.
        let score = (-(value * 8 + reply_total), -value, tie);
        if best.is_none_or(|(_, current)| score < current) {
            best = Some((coord, score));
        }
    }
    best.map(|(coord, _)| coord)
}

fn ranked_candidates(board: &Board, offsets: &[(i32, i32)], player: Player) -> Vec<RankedMove> {
    let Some(min_x) = offsets.iter().map(|(x, _)| *x).min() else {
        return Vec::new();
    };
    let max_x = offsets.iter().map(|(x, _)| *x).max().unwrap();
    let min_y = offsets.iter().map(|(_, y)| *y).min().unwrap();
    let max_y = offsets.iter().map(|(_, y)| *y).max().unwrap();
    let opponent_distances = distance_field(board, player.opponent(), DistanceMode::Geometric);
    let own_distances = distance_field(board, player, DistanceMode::Geometric);
    let mut candidates = Vec::new();
    for y in -min_y..board.height as i32 - max_y {
        for x in -min_x..board.width as i32 - max_x {
            let coord = Coord { x, y };
            if !valid_offsets(board, offsets, player, coord) {
                continue;
            }
            let mut approach = 0_i64;
            let mut expansion = 0_i64;
            for &(dx, dy) in offsets {
                let index = (y + dy) as usize * board.width + (x + dx) as usize;
                // Score the whole footprint; the overlapping own cell has distance zero.
                approach += i64::from(opponent_distances[index]);
                expansion += i64::from(own_distances[index]);
            }
            let score = (approach, -expansion, y, x);
            candidates.push((score, coord));
        }
    }
    candidates.sort_unstable_by_key(|(score, _)| *score);
    candidates
}

/// Called only after validating the occupied footprint against this board.
fn placed_board(board: &Board, offsets: &[(i32, i32)], player: Player, coord: Coord) -> Board {
    let mut next = board.clone();
    for &(dx, dy) in offsets {
        next.cells[(coord.y + dy) as usize][(coord.x + dx) as usize] = match player {
            Player::One => '@',
            Player::Two => '$',
        };
    }
    next
}

/// Estimate reachable territory after a move. Small boards use diagonal routes
/// and reward an arrival buffer: sparse pieces can cross diagonal gaps, and a
/// one-step lead alone is fragile. Large boards keep the cheaper original model.
fn territory_potential(board: &Board, player: Player) -> i64 {
    let (mode, margin) = if board.width * board.height <= CAUTIOUS_EXPANSION_CELLS {
        (DistanceMode::CautiousExpansion, 4_i64)
    } else if board.width * board.height <= FULL_EVALUATION_CELLS {
        (DistanceMode::Expansion, 4_i64)
    } else {
        (DistanceMode::Reachable, 1_i64)
    };
    let own = distance_field(board, player, mode);
    let opponent = distance_field(board, player.opponent(), mode);
    let mut score = 0;
    for (y, row) in board.cells.iter().enumerate() {
        for (x, &cell) in row.iter().enumerate() {
            let index = y * board.width + x;
            if player.is_own(cell) {
                score += margin;
            } else if player.is_opponent(cell) {
                score -= margin;
            } else {
                // i64 subtraction also handles one or both unreachable fields.
                score +=
                    (i64::from(opponent[index]) - i64::from(own[index])).clamp(-margin, margin);
            }
        }
    }
    score
}

pub fn format_coord(coord: Coord) -> String {
    format!("{} {}\n", coord.x, coord.y)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DistanceMode {
    Geometric,
    Reachable,
    Expansion,
    CautiousExpansion,
}

/// Shared multi-source BFS. Geometric fields ignore obstacles; reachable fields
/// cannot land on opposing territory. Expansion also permits diagonal routes
/// to model sparse pieces. Cautious expansion penalizes obstructed corners and
/// takes O(B log B); the other modes take O(B), where B is the board size.
fn distance_field(board: &Board, player: Player, mode: DistanceMode) -> Vec<i32> {
    let mut distances = vec![i32::MAX; board.width * board.height];
    let mut queue = VecDeque::new();
    for (y, row) in board.cells.iter().enumerate() {
        for (x, &cell) in row.iter().enumerate() {
            if player.is_own(cell) {
                let index = y * board.width + x;
                distances[index] = 0;
                queue.push_back(index);
            }
        }
    }
    if queue.is_empty() {
        if mode == DistanceMode::Geometric {
            distances.fill(0);
        }
        return distances;
    }
    if mode == DistanceMode::CautiousExpansion {
        return cautious_expansion(board, player, distances, queue);
    }
    while let Some(index) = queue.pop_front() {
        let x = index % board.width;
        let y = index / board.width;
        let neighbors = [
            (x > 0).then(|| index - 1),
            (x + 1 < board.width).then_some(index + 1),
            (y > 0).then(|| index - board.width),
            (y + 1 < board.height).then_some(index + board.width),
            (mode == DistanceMode::Expansion && x > 0 && y > 0).then(|| index - board.width - 1),
            (mode == DistanceMode::Expansion && x + 1 < board.width && y > 0)
                .then(|| index - board.width + 1),
            (mode == DistanceMode::Expansion && x > 0 && y + 1 < board.height)
                .then(|| index + board.width - 1),
            (mode == DistanceMode::Expansion && x + 1 < board.width && y + 1 < board.height)
                .then(|| index + board.width + 1),
        ];
        for neighbor in neighbors.into_iter().flatten() {
            if distances[neighbor] == i32::MAX
                && (mode == DistanceMode::Geometric
                    || !player
                        .is_opponent(board.cells[neighbor / board.width][neighbor % board.width]))
            {
                distances[neighbor] = distances[index] + 1;
                queue.push_back(neighbor);
            }
        }
    }
    distances
}

/// Dijkstra on compact boards. Diagonal growth past one opposing side cell
/// costs two steps; passing two needs a sparse piece and costs six steps.
fn cautious_expansion(
    board: &Board,
    player: Player,
    mut distances: Vec<i32>,
    sources: VecDeque<usize>,
) -> Vec<i32> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let mut queue: BinaryHeap<_> = sources
        .into_iter()
        .map(|index| Reverse((0, index)))
        .collect();
    while let Some(Reverse((distance, index))) = queue.pop() {
        if distance != distances[index] {
            continue;
        }
        let x = (index % board.width) as i32;
        let y = (index / board.width) as i32;
        for (dx, dy) in [
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 1),
        ] {
            let tx = x + dx;
            let ty = y + dy;
            if tx < 0 || ty < 0 || tx >= board.width as i32 || ty >= board.height as i32 {
                continue;
            }
            if player.is_opponent(board.cells[ty as usize][tx as usize]) {
                continue;
            }
            let diagonal = dx != 0 && dy != 0;
            let blocked_sides = if diagonal {
                i32::from(player.is_opponent(board.cells[y as usize][tx as usize]))
                    + i32::from(player.is_opponent(board.cells[ty as usize][x as usize]))
            } else {
                0
            };
            let cost = match blocked_sides {
                2 => 6,
                1 => 2,
                _ => 1,
            };
            let target = ty as usize * board.width + tx as usize;
            if distance + cost < distances[target] {
                distances[target] = distance + cost;
                queue.push(Reverse((distance + cost, target)));
            }
        }
    }
    distances
}
