#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;


use std::sync::Mutex;
use std::{thread, time};
use std::sync::Arc;



use std::net::TcpListener;
use std::net::TcpStream;


use physicsengine::MainGame;


fn main() {
    
    
    //matchmaker api listens on port 8000 (rocket default)
    //listens for client websocket connections on port 4000
    
    
    let thegame = Game::new();
    
    let mutexgame = Arc::new(Mutex::new( thegame ));
    
    
    
    //run the API that responds to requests from the matchmaker
    //about the state of the game
    {
        let mutexgamecopy = mutexgame.clone();
        
        thread::spawn(move || {
            rocket::ignite()
            .manage(mutexgamecopy)
            .mount("/", routes![ get_state, set_password, get_password, assign_player])
            .launch();
        });
    }
    
    
    
    //tick the game 30 times every second
    {
        let mutexgamecopy = mutexgame.clone();
        
        thread::spawn(move || {
            
            loop{
                
                //it shouldnt be WAIT 33 ms, but wait until its 
                //33 ms past the last time this was ticked
                let sleeptime = time::Duration::from_millis(32);
                thread::sleep( sleeptime );
                
                {
                    let mut game = mutexgamecopy.lock().unwrap();
                    
                    game.tick();    
                }
            }
        });
    }
    
    
    
    
    //for each websocket stream from a client
    //send it to the game
    {
        let webaddress = "0.0.0.0".to_string();
        
        let playerport = "4000";
        let playerlistener = TcpListener::bind(webaddress.clone() + ":" + playerport).unwrap();  
        
        
        for stream in playerlistener.incoming() {
            
            println!("incoming connection");
            
            let mutexgamecopy = mutexgame.clone();
            
            //accept a new websocket 10 times every second
            let sleeptime = time::Duration::from_millis(100);
            thread::sleep( sleeptime );
            
            
            use tungstenite::handshake::server::{Request, Response};
            use tungstenite::accept_hdr;
            
            if let Ok(stream) = stream{
                
                if let Ok(_) = stream.set_nonblocking(true){
                    
                    let callback = |req: &Request, mut response: Response| {
                        Ok(response)
                    };
                    

                    //exit if its not a websocket connection
                    if let Ok(mut websocket) = accept_hdr(stream, callback){
                        

                        //loop 10 times or until the connection succeeds
                        for x in 0..10{

                            let sleeptime = time::Duration::from_millis(500);
                            thread::sleep( sleeptime );

                            let mut game = mutexgamecopy.lock().unwrap();

                            //if the websocket is returned, it the connection wasnt accepted
                            if let Some(returnedwebsocket) = game.give_connection(websocket){

                                websocket = returnedwebsocket;
                            }
                            else{
                                break;
                            }

                        }


                    }
                }
            }
        }
    }
    
    
}



use rocket::State;





#[get("/get_state")]
fn get_state(state: State<Arc<Mutex<Game>>>) -> String {
    
    let game = state.inner();
    let game = game.lock().unwrap();
    
    game.get_state().to_string()
}


//get the password if it is set yet, otherwise return empty string
#[get("/get_password")]
fn get_password(state: State<Arc<Mutex<Game>>>) -> String {
    
    let game = state.inner();
    let game = game.lock().unwrap();
    
    game.get_password()
}


#[get("/set_password/<password>")]
fn set_password(password: String, state: State<Arc<Mutex<Game>>>) -> String{
    
    let game = state.inner();
    let mut game = game.lock().unwrap();
    
    game.set_password(password.clone());
    
    format!("the password was maybe (if not already set) set as {:?}", password).clone()
}

//assign a player to this game (add a player)
#[get("/assign_player")]
fn assign_player(state: State<Arc<Mutex<Game>>>) -> String {
    
    let game = state.inner();
    let mut game = game.lock().unwrap();
    
    game.assign_player();
    
    //return that it worked
    "Ok".to_string()

    //should really be checking somehwere to make sure I dont assign more than 2
}







//#[derive(Debug)]
struct Game{
    
    thegame: MainGame,
    
    password: Option<String>,
    
    player1websocket: Option< tungstenite::WebSocket<std::net::TcpStream>>,
    
    player2websocket: Option< tungstenite::WebSocket<std::net::TcpStream>>,
    
    
    //how many players have been assigned to this game
    assignedplayers: u8,
    
    //how many more ticks until you resend the state of the game to the players
    ticksuntilresendstate: i32,
}


impl Game{
    
    fn new() -> Game{
        
        Game{
            thegame: MainGame::new_two_player(),
            
            password: None,
            
            player1websocket: None,
            
            player2websocket: None,
            
            assignedplayers: 0,
            
            ticksuntilresendstate: 0,
        }
    }


    fn process_player_input(&mut self){

        //if the two websockets are connected
        if let Some(player1websocket) = self.player1websocket.as_mut(){
            
            if let Some(player2websocket) = self.player2websocket.as_mut(){

                if let Ok(receivedmessage) = player1websocket.read_message(){

                    let message = receivedmessage.to_string();

                    if let Ok(_) = self.thegame.receive_string_input(&1, message){                        
                        println!("receieved input from player 1");
                        self.ticksuntilresendstate = 0;
                    }

                }


                if let Ok(receivedmessage) = player2websocket.read_message(){

                    let message = receivedmessage.to_string();
                    
                    if let Ok(_) = self.thegame.receive_string_input(&2, message){    
                        println!("receieved input from player 2");                        
                        self.ticksuntilresendstate = 0;
                    }
                }


            }
        }
    }
    

    fn tick(&mut self){

        //process the incoming inputs of the players
        self.process_player_input();


        
        //if the two websockets are connected
        if let Some(player1websocket) = self.player1websocket.as_mut(){
            
            if let Some(player2websocket) = self.player2websocket.as_mut(){
                

                println!("game running and ticking");
                
                //tick the game
                self.thegame.tick();
                

                if self.ticksuntilresendstate <= 0{
                    
                    //get the state of the game
                    let gamestate = self.thegame.get_string_state();

                    println!("sednign game state updates to clients");
                    
                    //send it through both players websockets
                    {
                        
                        let p2message = tungstenite::Message::text(gamestate.clone());
                        if let Ok(sentsuccessfully) =  player2websocket.write_message(p2message){
                        }
                        
                        
                        let p1message = tungstenite::Message::text(gamestate);
                        if let Ok(sentsuccessfully) =  player1websocket.write_message(p1message){
                        }
                        
                    }

                    self.ticksuntilresendstate = 15;
                }

                self.ticksuntilresendstate += -1;
            }
        }


    }
    
    
    
    fn assign_player(&mut self) {
        
        self.assignedplayers += 1;
    }
    
    
    //get the state of the game
    fn get_state(&self)-> u8{
        
        //if its not responding to pings yet and isnt operating yet (0)   (assumed by default when theres no response)
        //if it hasnt had its password set yet (1)
        //get if it has a password set (2)  (aka, want new players to be assigned to this game)
        //get if it has both players registered (3) (aka, running and dont want new players to be assigned to this game)
        
        
        //if the password isnt set
        if self.password.is_none(){
            
            return 1;
        }
        //if less than 2 players have been assigned so far
        else if self.assignedplayers == 0 || self.assignedplayers == 1{
            
            return 2;
        }
        //and otherwise, return 3
        else{
            
            return 3;
        }
        
    }
    
    fn set_password(&mut self, password: String){
        
        //if the password isnt set yet, set it
        if self.password.is_none(){
            
            self.password  = Some(password);
        }
        
        //else do nothing
    }
    
    //get the password and return empty string if the password isnt set yet
    fn get_password(& self) -> String{
        
        if let Some(password) = &self.password{
            
            return password.clone();
        }
        else
        {
            return "".to_string();
        }
        
    }
    
    
    
    //a player wants to connect to the game
    //this method borrows and holds up the entire struct, so wait for the client to send the password
    //method before this function is called
    //return true if the input and password are valid and the player gets connected
    fn give_connection(&mut self, mut websocket: tungstenite::WebSocket<std::net::TcpStream>) -> Option<tungstenite::WebSocket<std::net::TcpStream>>{
        

        //if connected, send "connected to game as player 1" or 2
        
        
        //if theres a message
        if let Ok(msg) =  websocket.read_message(){            
            
            //if the message is a string
            if let Ok(textmsg) = msg.into_text(){
                
                //if the password is set yet
                if let Some(gamepassword) = &self.password{
                    
                    //if the message sent is the password
                    if &textmsg == gamepassword{
                        
                        //if player 1 doesnt exist, connect this websocket as player 1
                        if self.player1websocket.is_none(){


                            let p1message = tungstenite::Message::text("connected to game as player 1");
                            if let Ok(sentsuccessfully) =  websocket.write_message(p1message){
                            }
                            else{
                                return Some( websocket );
                            }

                            

                            self.player1websocket = Some(websocket);

                        }
                        //or if player 2 doesnt exist, connect this websocket as player 2
                        else if self.player2websocket.is_none(){


                            let p2message = tungstenite::Message::text("connected to game as player 2");
                            if let Ok(sentsuccessfully) =  websocket.write_message(p2message){
                            }
                            else{
                                return Some( websocket );
                            }


                            
                            self.player2websocket = Some(websocket);

                            //if there are 2 websockets connected, there are 2 players connected
                            self.assignedplayers = 2;

                        }

                        return None;
                    }
                }
            }
        }
        
        
        //otherwise, dont do anything, return and let the websocket connection fall out of scope

        return Some( websocket );
    }
    
    
}