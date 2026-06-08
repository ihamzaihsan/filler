use filler::{find_best_placement, format_coord, Coord, Player, TurnReader};
use std::io::{self, BufRead, Write};

fn main() {
    if let Err(error) = run() {
        eprintln!("filler: {error}");
        println!("0 0");
    }
}

fn run() -> io::Result<()> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut handshake = String::new();
    if input.read_line(&mut handshake)? == 0 {
        return Ok(());
    }
    let player = Player::from_exec_line(&handshake)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid player handshake"))?;
    let mut turns = TurnReader::new(input);
    let stdout = io::stdout();
    let mut output = stdout.lock();
    while let Some((board, piece)) = turns.read_turn()? {
        let coord = find_best_placement(&board, &piece, player).unwrap_or(Coord { x: 0, y: 0 });
        output.write_all(format_coord(coord).as_bytes())?;
        // The engine waits for a response before sending the next turn.
        output.flush()?;
    }
    Ok(())
}
