# ccp-websocket-server
hosts the matchmaking websocket server and the single game websocket servers



pull the ccp game

use that for the wasm_builder's dependancies

host the matchmaker

and the matchmaker creates a new single_server game on a certain port with a certain password



ideally, the only steps i would want to take to deploy this is a dockerfile
