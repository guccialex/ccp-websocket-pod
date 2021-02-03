FROM rust

WORKDIR /home

COPY . .

#pull the ccp-game from the server to be used as a dependancy for single_server
RUN svn co https://github.com/guccialex/ccp-game.git/trunk/chesspoker_package

WORKDIR /home/gamepod

RUN rustup update nightly
RUN rustup default nightly

RUN cargo build --release

CMD ROCKET_ENV=prod cargo run --release
