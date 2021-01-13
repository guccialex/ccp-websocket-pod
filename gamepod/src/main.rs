#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;


use std::sync::Mutex;
use std::{thread, time};
use std::sync::Arc;



use std::net::TcpListener;
use std::net::TcpStream;



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
            .mount("/", routes![ get_state, set_password, get_password])
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
            
            //already blocks when waiting for client to send the websocket password
            /*
            //accept a new websocket 10 times every second
            let sleeptime = time::Duration::from_millis(100);
            thread::sleep( sleeptime );
            */
            
            
            use tungstenite::handshake::server::{Request, Response};
            use tungstenite::accept_hdr;
            
            let stream = stream.unwrap();
            
            stream.set_nonblocking(true);
            
            let callback = |req: &Request, mut response: Response| {
                Ok(response)
            };
            
            //panic and exit if its not a websocket connection
            let mut websocket = accept_hdr(stream, callback).unwrap();
            
            
            //now that the websocket is established, wait 1000ms for the client to send the password
            let sleeptime = time::Duration::from_millis(1000);
            thread::sleep( sleeptime );
            
            
            let mut game = mutexgamecopy.lock().unwrap();
            
            game.give_connection(websocket);
            
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






#[derive(Debug)]
struct Game{
    
    password: Option<String>,
    
    player1websocket: Option< tungstenite::WebSocket<std::net::TcpStream>>,
    
    player2websocket: Option< tungstenite::WebSocket<std::net::TcpStream>>,
    
}


impl Game{
    
    fn new() -> Game{
        
        Game{
            password: None,
            
            player1websocket: None,
            
            player2websocket: None,
        }
    }
    
    fn tick(&mut self){
        
        
        println!("ticking");
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
        //if either of the websockets havent been set yet
        else if self.player1websocket.is_none() || self.player2websocket.is_none(){
            
            return 2;
        }
        //and otherwise, return 3
        else{
            
            return 3;
        }
        
    }
    
    
    //a player wants to connect to the game
    //this method borrows and holds up the entire struct, so wait for the client to send the password
    //method before this function is called
    fn give_connection(&mut self, mut websocket: tungstenite::WebSocket<std::net::TcpStream>){
        
        
        
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
                            
                            self.player1websocket = Some(websocket);
                            
                        }
                        //or if player 2 doesnt exist, connect this websocket as player 2
                        else if self.player2websocket.is_none(){
                            
                            self.player2websocket = Some(websocket);
                        }
                        
                    }
                }
            }
        }
        
        
        //otherwise, dont do anything, return and let the websocket connection fall out of scope
        
        
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
    
}