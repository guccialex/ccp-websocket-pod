FROM rust

WORKDIR /home

COPY . .

WORKDIR /home/gamepod


RUN cargo update
RUN rustup update nightly
RUN rustup default nightly



RUN cargo build --release
CMD cargo run --release
