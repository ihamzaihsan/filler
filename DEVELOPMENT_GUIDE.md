# Development Guide

Build the project, run individual matches, or check every robot on every map.

## 1. Build the Docker image

Install Docker with Linux containers enabled. From the repository root, run:

```sh
docker build --platform linux/amd64 -t filler docker_image
```

This compiles the Rust bot, match runner and replay generator. Rebuild the image after changing the code.

## 2. Open the container

```sh
docker run --rm -it filler
```

You are now in `/filler`. Run all remaining commands **inside this container**, unless marked as host commands. Type `exit` when finished.

## 3. Check the Rust code

```bash
cd /filler/solution
cargo fmt --check
cargo clippy --locked --offline --all-targets -- -D warnings
cargo test --locked --offline
cd /filler
```

## 4. Run one robot on one map

Regular mode shows the boards as the game progresses:

```bash
./linux_game_engine -f maps/map01 -p1 solution/target/release/filler -p2 linux_robots/bender
```

Quiet mode reduces the output. Add `-q`:

```bash
./linux_game_engine -f maps/map01 -p1 solution/target/release/filler -p2 linux_robots/bender -q
```

Change the map and opponent paths to check any matchup:

- Maps: `maps/map00`, `maps/map01`, `maps/map02`.
- Opponents: `linux_robots/bender`, `linux_robots/h2_d2`, `linux_robots/terminator`, `linux_robots/wall_e`.

Any map can be used with any opponent. To play as Player 2, swap the bot paths:

```bash
./linux_game_engine -f maps/map01 -p1 linux_robots/bender -p2 solution/target/release/filler -q
```

## 5. Run all robots on all maps

Choose the output mode first. For **quiet mode**:

```bash
mode=-q
```

For **regular mode**:

```bash
mode=
```

Then paste this loop into the container:

```bash
for map in maps/map00 maps/map01 maps/map02; do
  for bot in linux_robots/bender linux_robots/h2_d2 linux_robots/terminator linux_robots/wall_e; do
    echo "--- $map against $bot: solution as Player 1 ---"
    ./linux_game_engine -f "$map" -p1 solution/target/release/filler -p2 "$bot" $mode
    echo "--- $map against $bot: solution as Player 2 ---"
    ./linux_game_engine -f "$map" -p1 "$bot" -p2 solution/target/release/filler $mode
  done
done
```

This runs **24 matches**: all three maps against all four opponents, with your bot in both player positions. The engine chooses the seeds. Add `-s 1` to both engine commands to request seed 1.

## Optional: Save batch results

The match runner saves scores as CSV files and a text log for each game:

```bash
solution/target/release/audit --full --games 5 --both-roles --output /filler/results
```

This runs 120 matches using seeds 1 through 5 in both positions. It requires at least 80% wins for each map/opponent pairing. Add `--report-only` to save results without enforcing that threshold. Use `--random` for engine-selected seeds, or `--help` to see all options.

To keep the files, run the following commands in **a second host terminal**, from the repository root, before exiting the container:

```sh
docker ps
docker cp CONTAINER_ID:/filler/results ./filler-results
```

Replace `CONTAINER_ID` with the ID of your interactive `filler` container. Choose a new destination folder for each batch.

## Generate a graphical replay

Inside the container, record a match in regular mode and convert it to HTML:

```bash
./linux_game_engine -f maps/map01 -p1 solution/target/release/filler -p2 linux_robots/terminator > /filler/replay.txt
solution/target/release/replay /filler/replay.txt /filler/replay.html
```

In a second host terminal, find the container ID with `docker ps`, then copy the page before exiting the container:

```sh
docker cp CONTAINER_ID:/filler/replay.html ./replay.html
```

Open `replay.html` in your browser. It works offline and includes playback, frame navigation and a final-board view. You can also open the included `docs/demo.html` without running Docker.

## Continuous integration

[The workflow](.github/workflows/ci.yml) builds the image, checks formatting, runs Clippy and tests, then runs the 120-match batch described above. Match results are uploaded when available, including after a failed check.
