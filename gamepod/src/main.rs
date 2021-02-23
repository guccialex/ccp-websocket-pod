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
            .mount("/", routes![ get_players_in_game, get_password ])
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
        let mutexgamecopy = mutexgame.clone();
        
        thread::spawn(move ||{
            
            let webaddress = "0.0.0.0".to_string();
            
            let playerport = "4000";
            let playerlistener = TcpListener::bind(webaddress.clone() + ":" + playerport).unwrap();  
            
            
            for stream in playerlistener.incoming() {
                
                println!("incoming connection");
                
                let mutexgamecopy = mutexgamecopy.clone();
                
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
                                
                                let sleeptime = time::Duration::from_millis(1000);
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
        });
    }
    
    
    
    //loop until the mutex game is poisoned, then end this pod by panicing
    {
        let mutexgamecopy = mutexgame.clone();
        
        loop{
            let sleeptime = time::Duration::from_millis(2000);
            thread::sleep( sleeptime );
            
            if let Ok(_) = mutexgamecopy.lock(){

                //not poisoned
            }
            else{

                panic!("Poisoned Main struct. End the pod");
            }
        }
    }
    
    
}



use rocket::State;





#[get("/get_players_in_game")]
fn get_players_in_game(state: State<Arc<Mutex<Game>>>) -> String {
    
    println!("getting players in game requested");
    let game = state.inner();
    let game = game.lock().unwrap();
    
    game.get_players_in_game().to_string()
}


//get the password if it is set yet, otherwise return empty string
#[get("/get_password")]
fn get_password(state: State<Arc<Mutex<Game>>>) -> String {
    
    println!("getting request for password");
    let game = state.inner();
    let game = game.lock().unwrap();
    
    game.get_password()
}






//#[derive(Debug)]
struct Game{
    
    thegame: MainGame,
    
    password: String,
    
    player1websocket: Option< tungstenite::WebSocket<std::net::TcpStream>>,
    
    player2websocket: Option< tungstenite::WebSocket<std::net::TcpStream>>,
    
    //how many more ticks until you resend the state of the game to the players
    ticksuntilresendstate: i32,
    
    //ticks until end
    ticksuntilpanic: i32,
}


impl Game{
    
    fn new() -> Game{
        
        
        use rand::{distributions::Alphanumeric, Rng};
        
        let passwordtoset = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect();
        
        
        Game{
            thegame: MainGame::new_two_player(),
            
            password: passwordtoset,
            
            player1websocket: None,
            
            player2websocket: None,
            
            ticksuntilresendstate: 0,
            
            ticksuntilpanic: 100000,
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
        


        //if player 1 or 2 arent connected, adn therefore teh game isnt ticking
        //(which would panic after a sufficient amount of time by itself)
        //tick down ticks until panic, and panic if less than 0
        if self.player1websocket.is_none() || self.player2websocket.is_none(){

            self.ticksuntilpanic = self.ticksuntilpanic - 1;
            if self.ticksuntilpanic <= 0{   
                panic!("Ahhh. this pod has been living long enough");
            }
        }
        
        
        
        //process the incoming inputs of the players
        self.process_player_input();
        
        
        //if the two websockets are connected
        if let Some(player1websocket) = self.player1websocket.as_mut(){
            
            if let Some(player2websocket) = self.player2websocket.as_mut(){
                
                
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
                    
                    self.ticksuntilresendstate = 30;
                }
                
                self.ticksuntilresendstate += -1;
            }
        }
        
        
        
        
        //check if either websocket is still connected
        //if one has been disconnected for longer than... a while, panic
        
        
    }
    
    
    
    //get the players in the game
    fn get_players_in_game(&self)-> u8{
        
        let mut playersconnected = 0;
        
        if self.player1websocket.is_some(){
            playersconnected += 1;
        }

        if self.player2websocket.is_some(){
            playersconnected += 1;
        }
        
        println!("the players in game {:?}", playersconnected);

        playersconnected
    }
    
    
    
    
    //get the password and return empty string if the password isnt set yet
    fn get_password(& self) -> String{
        
        return self.password.clone();
    }
    
    
    
    //return NONE if the connection succeeded
    //return the websocket if the connection failed
    fn give_connection(&mut self, mut websocket: tungstenite::WebSocket<std::net::TcpStream>) -> Option<tungstenite::WebSocket<std::net::TcpStream>>{
        
        
        //if connected, send "connected to game as player 1" or 2
        
        
        //if theres a message
        if let Ok(msg) =  websocket.read_message(){            
            
            //if the message is a string
            if let Ok(textmsg) = msg.into_text(){
                
                //if the message sent is the password
                if &textmsg == &self.password{
                    
                    //if player 1 doesnt exist, connect this websocket as player 1
                    if self.player1websocket.is_none(){


                        let p1message = tungstenite::Message::text("connected to game as player 1");
                        if let Ok(sentsuccessfully) =  websocket.write_message(p1message){
                            
                            self.player1websocket = Some(websocket);
                            return None;
                        }
                        else{
                            return Some( websocket );
                        }                        
                        
                    }
                    //or if player 2 doesnt exist, connect this websocket as player 2
                    else if self.player2websocket.is_none(){
                        

                        let p2message = tungstenite::Message::text("connected to game as player 2");
                        if let Ok(sentsuccessfully) =  websocket.write_message(p2message){
                            
                            self.player2websocket = Some(websocket);
                            return None;
                        }
                        else{
                            return Some( websocket );
                        }
                    }
                }
            }
        }
        
        
        //otherwise return teh websocket that wasnt set as either player 1 or 2
        return Some( websocket);
    }
    
    
}