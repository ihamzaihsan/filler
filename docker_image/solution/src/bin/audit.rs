//! Seeded and engine-randomized matches with raw logs, CSV scores, and audit thresholds.
use std::env;
use std::fmt::Write as FmtWrite;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

struct Options {
    output: PathBuf,
    bot: String,
    games: u32,
    start_seed: u32,
    both_roles: bool,
    bonus: bool,
    full: bool,
    random: bool,
    opponent: Option<String>,
    report_only: bool,
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(error) => {
            eprintln!("audit: {error}");
            std::process::exit(2);
        }
    }
}

fn options() -> io::Result<Options> {
    parse_options(env::args().skip(1))
}

fn parse_options(args: impl Iterator<Item = String>) -> io::Result<Options> {
    let mut options = Options {
        output: "results".into(),
        bot: "solution/target/release/filler".into(),
        games: 5,
        start_seed: 1,
        both_roles: false,
        bonus: false,
        full: false,
        random: false,
        opponent: None,
        report_only: false,
    };
    let mut args = args;
    let mut explicit_seed = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => options.output = value(&mut args, &arg)?.into(),
            "--bot" => options.bot = value(&mut args, &arg)?,
            "--games" => options.games = number(&mut args, &arg)?,
            "--start-seed" => {
                options.start_seed = number(&mut args, &arg)?;
                explicit_seed = true;
            }
            "--both-roles" => options.both_roles = true,
            "--bonus" => options.bonus = true,
            "--full" => options.full = true,
            "--random" => options.random = true,
            "--opponent" => options.opponent = Some(value(&mut args, &arg)?),
            "--report-only" => options.report_only = true,
            "--help" | "-h" => {
                println!(
                    "Run from /filler (the game environment root).\n\
                    audit [--output DIR] [--bot PATH] [--games N] [--start-seed N]\n\
                    [--both-roles] [--bonus] [--full] [--random] [--opponent NAME] [--report-only]\n\
                    Default: five alternating-role games for each required audit pairing.\n\
                    --bonus adds terminator on map01; --full uses all map/opponent pairs.\n\
                    --both-roles plays each seed as p1 and p2.\n\
                    --random omits the seed flag: the engine chooses a fresh seed each match.\n\
                    --full --opponent NAME tests that opponent on all three maps.\n\
                    Exit codes: 0 pass, 1 below 80% wins, 2 engine/tool error.\n\
                    --report-only records scores without enforcing the win threshold."
                );
                std::process::exit(0);
            }
            _ => return Err(invalid(&format!("unknown option: {arg}"))),
        }
    }
    if options.random && explicit_seed {
        return Err(invalid("--random cannot be combined with --start-seed"));
    }
    if let Some(opponent) = options.opponent.as_deref() {
        if !["wall_e", "h2_d2", "bender", "terminator"].contains(&opponent) {
            return Err(invalid(
                "opponent must be wall_e, h2_d2, bender, or terminator",
            ));
        }
        if !options.full {
            return Err(invalid("--opponent requires --full to select all maps"));
        }
    }
    if options.games == 0
        || options.games > 1000
        || options.start_seed.checked_add(options.games).is_none()
    {
        return Err(invalid(
            "games must be 1..=1000 and seeds must not overflow",
        ));
    }
    if options.bot.contains([',', '\n', '\r']) {
        return Err(invalid("bot path cannot contain CSV delimiters"));
    }
    Ok(options)
}

fn value(args: &mut impl Iterator<Item = String>, flag: &str) -> io::Result<String> {
    args.next()
        .ok_or_else(|| invalid(&format!("missing value for {flag}")))
}

fn number(args: &mut impl Iterator<Item = String>, flag: &str) -> io::Result<u32> {
    value(args, flag)?
        .parse()
        .map_err(|_| invalid(&format!("invalid number for {flag}")))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn run() -> io::Result<bool> {
    let options = options()?;
    let mut pairs = vec![("map00", "wall_e"), ("map01", "h2_d2"), ("map02", "bender")];
    if options.full {
        pairs.clear();
        for map in ["map00", "map01", "map02"] {
            for opponent in ["wall_e", "h2_d2", "bender", "terminator"] {
                pairs.push((map, opponent));
            }
        }
    } else if options.bonus {
        pairs.push(("map01", "terminator"));
    }
    if let Some(opponent) = options.opponent.as_deref() {
        pairs.retain(|(_, name)| *name == opponent);
    }
    fs::create_dir_all(&options.output)?;
    let mut csv = File::create(options.output.join("matches.csv"))?;
    writeln!(
        csv,
        "map,opponent,seed,bot_player,bot_score,opponent_score,result,elapsed_ms,game,seed_mode"
    )?;
    let mut summary = String::from("map,opponent,wins,draws,losses,games,passed\n");
    let mut all_passed = true;
    for (map, opponent) in pairs {
        let mut wins = 0_u32;
        let mut draws = 0_u32;
        let mut losses = 0_u32;
        for game in 0..options.games {
            let seed = (!options.random).then_some(options.start_seed + game);
            let roles: &[u32] = if options.both_roles {
                &[1, 2]
            } else if game % 2 == 0 {
                &[1]
            } else {
                &[2]
            };
            for &role in roles {
                let opponent_path = format!("linux_robots/{opponent}");
                let (p1, p2) = if role == 1 {
                    (options.bot.as_str(), opponent_path.as_str())
                } else {
                    (opponent_path.as_str(), options.bot.as_str())
                };
                let label = match seed {
                    Some(seed) => format!("seed{seed}"),
                    None => format!("game{}", game + 1),
                };
                let log = options
                    .output
                    .join(format!("{map}-{opponent}-{label}-p{role}.txt"));
                let start = Instant::now();
                let (score1, score2, actual_seed) = engine_match(map, p1, p2, seed, &log)?;
                let elapsed = start.elapsed().as_millis();
                let (bot_score, opponent_score) = if role == 1 {
                    (score1, score2)
                } else {
                    (score2, score1)
                };
                let result = match bot_score.cmp(&opponent_score) {
                    std::cmp::Ordering::Greater => {
                        wins += 1;
                        "win"
                    }
                    std::cmp::Ordering::Equal => {
                        draws += 1;
                        "draw"
                    }
                    std::cmp::Ordering::Less => {
                        losses += 1;
                        "loss"
                    }
                };
                let seed_mode = if options.random {
                    "engine-default"
                } else {
                    "specified"
                };
                writeln!(csv, "{map},{opponent},{actual_seed},{role},{bot_score},{opponent_score},{result},{elapsed},{},{seed_mode}", game + 1)?;
                csv.flush()?;
                println!("{map} vs {opponent}, {label}, seed {actual_seed} ({seed_mode}), p{role}: {bot_score}-{opponent_score} ({result})");
            }
        }
        let games = wins + draws + losses;
        let passed = wins * 5 >= games * 4;
        all_passed &= passed;
        writeln!(
            &mut summary,
            "{map},{opponent},{wins},{draws},{losses},{games},{passed}"
        )
        .expect("writing a String cannot fail");
        println!(
            "{map} vs {opponent}: {wins}/{games} wins; {}",
            if passed { "PASS" } else { "FAIL" }
        );
    }
    fs::write(options.output.join("summary.csv"), summary)?;
    Ok(options.report_only || all_passed)
}

fn engine_args(map: &str, p1: &str, p2: &str, seed: Option<u32>) -> Vec<String> {
    let mut args = vec![
        "-f".into(),
        format!("maps/{map}"),
        "-p1".into(),
        p1.into(),
        "-p2".into(),
        p2.into(),
    ];
    if let Some(seed) = seed {
        args.extend(["-s".into(), seed.to_string()]);
    }
    args.extend(["-t".into(), "10".into(), "-q".into()]);
    args
}

fn engine_match(
    map: &str,
    p1: &str,
    p2: &str,
    seed: Option<u32>,
    log: &Path,
) -> io::Result<(u32, u32, i64)> {
    let args = engine_args(map, p1, p2, seed);
    let mut output = File::create(log)?;
    writeln!(output, "./linux_game_engine {}", args.join(" "))?;
    output.flush()?;
    let mut child = Command::new("./linux_game_engine")
        .args(args)
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(output)
        .spawn()?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() > Duration::from_secs(120) {
            child.kill()?;
            child.wait()?;
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("match exceeded 120s: {}", log.display()),
            ));
        }
        thread::sleep(Duration::from_millis(20));
    };
    if !status.success() {
        return Err(io::Error::other(format!(
            "engine exited {status}; see {}",
            log.display()
        )));
    }
    let text = fs::read_to_string(log)?;
    let (p1_score, p2_score, actual_seed) = parse_engine_output(&text).ok_or_else(|| {
        io::Error::other(format!(
            "invalid final scores or seed; see {}",
            log.display()
        ))
    })?;
    if seed.is_some_and(|seed| i64::from(seed) != actual_seed) {
        return Err(io::Error::other(format!(
            "engine seed differs from requested seed; see {}",
            log.display()
        )));
    }
    Ok((p1_score, p2_score, actual_seed))
}

fn parse_engine_output(text: &str) -> Option<(u32, u32, i64)> {
    let (p1, p2) = parse_scores(text)?;
    let seed = text
        .lines()
        .find_map(|line| line.strip_prefix("seed:"))?
        .trim()
        .parse()
        .ok()?;
    Some((p1, p2, seed))
}

fn parse_scores(text: &str) -> Option<(u32, u32)> {
    let mut p1 = None;
    let mut p2 = None;
    let mut winner = None;
    for line in text.lines() {
        if line.starts_with("Player1 (") {
            p1 = line.rsplit_once(':')?.1.trim().parse().ok();
        }
        if line.starts_with("Player2 (") {
            p2 = line.rsplit_once(':')?.1.trim().parse().ok();
        }
        match line.trim() {
            "Player1 won!" => winner = Some(1),
            "Player2 won!" => winner = Some(2),
            _ => {}
        }
    }
    let (p1, p2): (u32, u32) = (p1?, p2?);
    match (p1.cmp(&p2), winner) {
        (std::cmp::Ordering::Greater, Some(1))
        | (std::cmp::Ordering::Less, Some(2))
        | (std::cmp::Ordering::Equal, None) => Some((p1, p2)),
        _ => None,
    }
}
