

#pull the ccp-game from the server to be used as a dependancy for single_server
svn co https://github.com/guccialex/ccp-game.git/trunk/chesspoker_package


#build the single_server first so players can connect without waiting for the first build
cd single_server
cargo build --release


#run the matchmaker server
cd ..
cd matchmaker

cargo run --release
