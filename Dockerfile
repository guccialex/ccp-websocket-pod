FROM rust

WORKDIR /home

COPY . .

WORKDIR /home/gamepod

RUN rustup update nightly
RUN rustup default nightly

RUN cargo build --release

#ROCKET_ENV=prod

CMD cargo run --release
