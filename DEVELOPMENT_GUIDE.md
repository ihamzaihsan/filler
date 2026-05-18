# Development Guide

This guide provides step-by-step instructions on how to set up the environment, build the code, and run matches for the filler project.

## 1. Build the Docker Image

First, navigate to the `docker_image` directory and build the Docker container:

```bash
cd docker_image
docker build -t filler .
```

## 2. Run the Docker Container

Next, run the container interactively. This command mounts your local `solution` directory into the container so that your code changes sync automatically. Make sure you are in the `docker_image` directory when you run this:

```bash
docker run -v "$(pwd)/solution":/filler/solution -it filler
```

You are now inside the container, currently in the `/filler` directory.

## 3. Build the Robot

Navigate to your mounted solution folder to compile the Rust project:

```bash
cd solution
cargo build --release
```

## 4. Run Tests

To ensure everything is working correctly, run the cargo tests:

```bash
cargo test
```

## 5. Run a Match

Go back to the root of the game environment:

```bash
cd /filler
```

Start a match on `map01` between your newly compiled solution (Player 1) and the provided `bender` robot (Player 2):

```bash
./linux_game_engine -f maps/map01 -p1 solution/target/release/filler -p2 linux_robots/bender
```

### Quiet Output

If you prefer to run the match with less console output, you can append the `-q` flag:

```bash
./linux_game_engine -f maps/map01 -p1 solution/target/release/filler -p2 linux_robots/bender -q
```

### Run All Scenarios (All Maps & All Opponents)

To test your robot against all opponents (`bender`, `h2_d2`, `terminator`, `wall_e`) on all maps (`map00`, `map01`, `map02`), you can run this single compound command inside the container:

```bash
for map in maps/map00 maps/map01 maps/map02; do for bot in linux_robots/bender linux_robots/h2_d2 linux_robots/terminator linux_robots/wall_e; do echo "--- Testing on $map against $bot ---"; ./linux_game_engine -f "$map" -p1 solution/target/release/filler -p2 "$bot" -q; done; done
