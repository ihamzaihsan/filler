FROM rust:1.99.0-bookworm

WORKDIR /filler
COPY maps/ maps/
COPY linux_robots/ linux_robots/
COPY linux_game_engine ./linux_game_engine
COPY solution/ solution/

RUN rustup component add rustfmt clippy \
    && chmod +x linux_game_engine linux_robots/* \
    && ln -s linux_game_engine game_engine \
    && ln -s linux_robots robots \
    && sed -i 's/\r$//' maps/* \
    && cargo build --release --locked --offline --manifest-path solution/Cargo.toml

ENTRYPOINT ["bash"]
