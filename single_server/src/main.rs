use physicsengine::MainGame;
use std::sync::Arc;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread::spawn;
use tungstenite::accept_hdr;
use tungstenite::handshake::server::{Request, Response};
use tungstenite::server::accept;
use std::collections::HashMap;
use std::collections::HashSet;
use tungstenite::{connect, Message};

use  std::sync::Mutex;
use std::{thread, time};


use std::env;


fn main() {
    println!("Hello, world!");


    //the command line arguments
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    
    let gamepassword = args[2].clone();
    
    let webaddress = "127.0.0.1".to_string();
    let gameport = &args[1];

    let listener = TcpListener::bind(webaddress + ":" + gameport).unwrap();
    
    
    //tell agones that the game is ready to accept player connections
    let thegame = Game::new();
    
    let mutexgame = Arc::new(Mutex::new( thegame ));
    
    
    
    
    //tick the game 30 times a second
    let mutexgamecopy = mutexgame.clone();
    spawn(move || {
        
        loop{

            
            //it shouldnt be WAIT 33 ms, but wait until its 
            //33 ms past the last time this was ticked
            let sleeptime = time::Duration::from_millis(32);
            thread::sleep( sleeptime );
            
            
            //taking ownership of the "games" list
            //to tick the game
            {
                let mut game = mutexgamecopy.lock().unwrap();
                
                game.tick();    
            }
            
        }
        
        
    });
    
    
    
    //for each websocket stream this server gets
    for stream in listener.incoming() {
        
        
        //accept a new websocket 10 times every second
        let sleeptime = time::Duration::from_millis(100);
        thread::sleep( sleeptime );
        
        let mutexgamecopy = mutexgame.clone();

        let gamepasswordcopy = gamepassword.clone();
        
        
        //spawn a new thread for the connection
        spawn(move || {
            
            let stream = stream.unwrap();
            
            handle_connection(stream, mutexgamecopy, gamepasswordcopy);            
        });        
    }
    
    
    
    
    
    
    
}



//handle a connection for the game
fn handle_connection(mut stream: TcpStream, game: Arc< Mutex< Game >>, password: String){
    
    
    
    //the password needed to connect to the game as a certain player
    let password = password;
    
    
    
    stream.set_nonblocking(true);
    
    let callback = |req: &Request, mut response: Response| {
        Ok(response)
    };
    
    //panic and exit the thread if its not a websocket connection
    let mut websocket = accept_hdr(stream, callback).unwrap();
    
    
    
    //wait 2000 millis
    let sleeptime = time::Duration::from_millis(2000);
    thread::sleep( sleeptime );
    


    
    
    //if theres a message
    //only read the first message, if the first message isnt used
    
    if let Ok(msg) =  websocket.read_message(){
        
        println!("the message received: {:?}", msg);
        
        
        //if the message im receiving is a string
        if let Ok(textmsg) = msg.into_text(){
            


            //if its the password
            if textmsg == password{
                
                if let Ok(unlockedgame) = &mut game.lock(){


                    if unlockedgame.player1websocket.is_none(){
                        //if player 1 doesnt exist, connect this websocket as player 1
                        unlockedgame.connect_player1(websocket);

                    }
                    //or if player 2 doesnt exist, connect this websocket as player 2
                    else if unlockedgame.player2websocket.is_none(){

                        //if player 1 doesnt exist, connect this websocket as player 1
                        unlockedgame.connect_player2(websocket);
                    }
                
                }


            }

            
            
            //if its not the password for either, do nothing
            //just let the websocket connection end
            
            
        }
        
        
    }
    
    
    
}





//a single game
struct Game{
    
    thegame: MainGame,
    
    //if everything about the game is valid enough for it to tick
    gameon: bool,
    
    
    player1active: bool,
    player2active: bool,
    
    
    player1websocket: Option< tungstenite::WebSocket<std::net::TcpStream> >,
    player2websocket: Option< tungstenite::WebSocket<std::net::TcpStream> >,
    
    
    totalticks: u32,

    //if I received an input from a player last tick, send an update method
    tosendupdate: bool,
    
    
}

impl Game{
    
    fn new() -> Game{
        
        
        Game{
            
            thegame: MainGame::new_two_player(),
            
            gameon: false,
            
            player1active: false,
            player2active: false,
            
            player1websocket: None,
            
            player2websocket: None,
            
            totalticks: 0,

            tosendupdate: false,
            
        }
        
    }
    
    
    fn connect_player1(&mut self, websocket: tungstenite::WebSocket<std::net::TcpStream> ){
        
        //if player 1 does not have their websocket connection set
        
        if self.player1websocket.is_none(){
            self.player1websocket = Some(websocket);
            
            self.player1active = true;
            
            
            
            let player1msg = Message::text("connected to game as player 1");
            self.player1websocket.as_mut().unwrap().write_message(player1msg).unwrap();
        }
        
        
        
    }
    
    
    fn connect_player2(&mut self, websocket: tungstenite::WebSocket<std::net::TcpStream>){
        
        
        //if player 2 does not have their websocket connection set
        if self.player2websocket.is_none(){
            self.player2websocket = Some(websocket);
            
            self.player2active = true;
            
            
            let player2msg = Message::text("connected to game as player 2");
            self.player2websocket.as_mut().unwrap().write_message(player2msg).unwrap();
            
        }
        
        
    }
    
    
    fn tick(&mut self){
        
        
        //set the game to be on if both players are active
        //and off if either player is inactive
        if self.player1active && self.player2active{
            self.gameon = true;
        }
        else{
            //THIS SHOULD BE FALSE
            //BUT IM SETTING IT TO TRUE FOR TESTING
            self.gameon = true;
        }
        
        
        //if the game state is valid to tick it
        if self.gameon{
            
            self.totalticks += 1;
            
            //tick the game
            self.thegame.tick();
            
            
            //receive player 1's queued input if there is any
            {
                
                use physicsengine::PlayerInput;
                
                if let Some(socket) = &mut self.player1websocket{
                    
                    if let Ok(receivedmessage) = socket.read_message(){
                        
                        self.tosendupdate = true;

                        let message = receivedmessage.to_string();
                        
                        //convert this to a player input
                        //if you can
                        if let Ok(playerinput) = serde_json::from_str::<PlayerInput>(&message){

                            //give the player input to the game
                            self.thegame.receive_input(1, playerinput);
                        }
                        
                        
                    }
                }
                
            }
            //receive player 2's queued input if there is any
            {

                use physicsengine::PlayerInput;
                
                if let Some(socket) = &mut self.player2websocket{
                    
                    if let Ok(receivedmessage) = socket.read_message(){
                        
                        self.tosendupdate = true;

                        let message = receivedmessage.to_string();


                        //convert this to a player input
                        //if you can
                        if let Ok(playerinput) = serde_json::from_str::<PlayerInput>(&message){

                            //give the player input to the game
                            self.thegame.receive_input(2, playerinput);
                        }
                        
                    }
                }
                
            }
            
            
            
            
            //send the states of the game through the websocket
            //if the websocket is open this tick
            if self.totalticks % 45 == 0 || self.tosendupdate{
                
                let gamebinto1 = bincode::serialize(&self.thegame).unwrap();
                let vecofchar = gamebinto1.iter().map(|b| *b as char).collect::<Vec<_>>();
                let stringmessage = vecofchar.iter().collect::<String>();
                let player1msg = Message::text(stringmessage);
                if let Some(thing) = self.player1websocket.as_mut(){

                    if let Ok(sentsuccessfully) =  thing.write_message(player1msg){

                    }
                    else{
                        //send failed
                        //player 1 probably disconnected
                    }

                }
                
                
                let gamebinto2 = bincode::serialize(&self.thegame).unwrap();
                let vecofchar = gamebinto2.iter().map(|b| *b as char).collect::<Vec<_>>();
                let stringmessage = vecofchar.iter().collect::<String>();
                let player2msg = Message::text(stringmessage);
                if let Some(thing) = self.player2websocket.as_mut(){

                    if let Ok(sentsuccessfully) =  thing.write_message(player2msg){

                    }
                    else{
                        //send failed
                        //player 2 probably disconnected               
                    }
                }


                self.tosendupdate = false;
                
            }



        }
    }
}



