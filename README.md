# ccp-websocket-server
hosts the matchmaking websocket server and the single game websocket servers



pull the ccp game

use that for the wasm_builder's dependancies

host the matchmaker

and the matchmaker creates a new single_server game on a certain port with a certain password


The ports exposed by the dockerfile will be 3050 where the matchmaker runs

and ports 12000 to 13000 where the single servers will be hosted


RUN WITH OPTIONS
-p 12000-13000:12000-13000 -p 3050:3050


build with the dockerfile

then run that image or publish it to some repository to run it on gcp
