//! Converts a non-quiet engine transcript into a standalone browser replay.
use filler::{parse_board, parse_dimensions, Board};
use std::env;
use std::fs;
use std::io;

const TEMPLATE: &str = include_str!("../../assets/replay.html");

fn main() {
    if let Err(error) = run() {
        eprintln!("replay: {error}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: replay INPUT.txt OUTPUT.html (record engine without -q)",
        ));
    }
    let frames = parse_frames(&fs::read_to_string(&args[0])?)?;
    let json = format!(
        "[{}]",
        frames
            .iter()
            .map(|board| {
                let rows: Vec<_> = board
                    .cells
                    .iter()
                    .map(|row| {
                        // Board parsing permits only . @ $ a s, so rows cannot contain JSON/HTML escapes.
                        format!("\"{}\"", row.iter().collect::<String>())
                    })
                    .collect();
                format!("[{}]", rows.join(","))
            })
            .collect::<Vec<_>>()
            .join(",")
    );
    fs::write(&args[1], TEMPLATE.replace("/* REPLAY_DATA */ []", &json))?;
    println!("Saved {} snapshots to {}", frames.len(), args[1]);
    Ok(())
}

fn parse_frames(input: &str) -> io::Result<Vec<Board>> {
    let mut lines = input.lines();
    let mut frames = Vec::new();
    while let Some(header) = lines.next() {
        if !header.starts_with("Anfield ") {
            continue;
        }
        let (_, height) = parse_dimensions(header, "Anfield")
            .ok_or_else(|| bad_replay("invalid board dimensions"))?;
        lines
            .next()
            .ok_or_else(|| bad_replay("missing column ruler"))?;
        let rows: Vec<_> = lines.by_ref().take(height).map(String::from).collect();
        let board = parse_board(header, &rows).ok_or_else(|| bad_replay("invalid board rows"))?;
        if frames
            .first()
            .is_some_and(|first: &Board| first.width != board.width || first.height != board.height)
        {
            return Err(bad_replay("board dimensions changed during match"));
        }
        frames.push(board);
    }
    if frames.is_empty() {
        return Err(bad_replay("no snapshots found; record a match without -q"));
    }
    Ok(frames)
}

fn bad_replay(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
