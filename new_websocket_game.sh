#!/bin/bash

#create a new websocket game
#at this port and with this password
#there is no checking done here that the port requested is valid


echo "you want a websocket game on port $1 with the password $2"


cd single_server

cargo run --release $1 $2
