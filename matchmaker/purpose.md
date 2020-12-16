the purpose of the matchmaker is to get requests from the frontend
asking to connect to either
a public game
create a private game
or connect to a private game with a certain password


the matchmaker runs a new single_server if it doesnt exist
that opens on a unique port
and gives the port for that single server back to the thing requesting it
