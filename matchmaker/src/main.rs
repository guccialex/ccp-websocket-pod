
use std::net::TcpListener;
use std::net::TcpStream;
use std::{thread, time};

use tungstenite::{Message};
use std::process::Command;

use std::sync::Arc;
use  std::sync::Mutex;


use tungstenite::handshake::server::{Request, Response};
use tungstenite::accept_hdr;

fn main() {
    
    
    //open a websocket connection with any server 
    let webaddress = "127.0.0.1".to_string();
    let gameport = 3050.to_string();


    
    let listener = TcpListener::bind(webaddress + ":" + &gameport).unwrap();
    
    let mutexmain = Arc::new(Mutex::new(Main::new()));
    
    
    
    
    
    //for each websocket stream this server gets
    for stream in listener.incoming() {
        
        
        //accept a new websocket 10 times every second
        let sleeptime = time::Duration::from_millis(100);
        thread::sleep( sleeptime );
        

        let copiedmutexmain = mutexmain.clone();


        println!("player connected to the server");


        
        //spawn a new thread for the connection
        thread::spawn(move || {
            
            let stream = stream.unwrap();

            stream.set_nonblocking(true);
    
            let callback = |req: &Request, mut response: Response| {
                Ok(response)
            };
    
            //panic and exit the thread if its not a websocket connection
            let websocket = accept_hdr(stream, callback).unwrap();

            
            handle_connection( copiedmutexmain, websocket );
            
        });    
        
    }
    
}



//handle the connection

fn handle_connection( mutexmain: std::sync::Arc<std::sync::Mutex<Main>>,  mut newsocket: tungstenite::WebSocket<std::net::TcpStream>){
    
    


    
    let mut loopnumber = 0;
    
    //loop until i get a message to connect to a certain game
    loop{
        
        
        //wait 1 second
        let sleeptime = time::Duration::from_millis(1000);
        thread::sleep( sleeptime );
        
        
        //if this has looped more than 2000 times break
        if loopnumber > 2000{
            break;
        }
        else{
            loopnumber += 1;
        }
        
        

        if let Ok(receivedmessage) = newsocket.read_message(){

            //unlock the mutex main while handling this message
            let mut main = mutexmain.lock().unwrap();

            let mut connectsucceeded = false;
            
            let message = receivedmessage.to_string();

            println!("i received this message through the websocket {}", message);
            
            //convert this to a request to connect to a certain game
            if let Ok(gametoconnectto) = serde_json::from_str::<GameToConnectTo>(&message){

                println!("I received this object from the game {:?}", gametoconnectto);
                
                if let GameToConnectTo::joinpublicgame = gametoconnectto{
                    

                    let matchpassword = MatchPassword::Public;
                    
                    
                    //if an open public game exists
                    //send the port of it back
                    if let Some(publicgameport) = main.openmatches.get( &matchpassword ){
                        
                        //send the port through the websocket
                        connectsucceeded = Main::send_connected_message_through_websocket(&mut newsocket, publicgameport, &main.publicpassword);
                        
                        //remove this match from the list of open matches
                        main.openmatches.remove( &matchpassword );
                    }
                    //if not
                    //create it and send the port of it back
                    else{
                        
                        let port = Main::create_game(main.publicpassword.clone());
                        
                        main.openmatches.insert( matchpassword, port );
                        
                        //send the port through the websocket
                        connectsucceeded = Main::send_connected_message_through_websocket(&mut newsocket, &port, &main.publicpassword);
                    }
                    
                }
                else if let GameToConnectTo::createprivategame = gametoconnectto{
                    
                    use std::iter;
                    use rand::{Rng, thread_rng};
                    use rand::distributions::Alphanumeric;
                    
                    let mut rng = thread_rng();
                    let gamepassword: String = iter::repeat(())
                    .map(|()| rng.sample(Alphanumeric))
                    .take(4)
                    .collect();
                    
                    
                    let port = Main::create_game(gamepassword.clone());
                    
                    //add it to the list of open games
                    main.openmatches.insert( MatchPassword::Private(gamepassword.clone()), port );
                    
                    //send the port through the websocket
                    connectsucceeded = Main::send_connected_message_through_websocket(&mut newsocket, &port, &gamepassword);
                    
                }
                else if let GameToConnectTo::joinprivategame(privategamepassword) = gametoconnectto{
                    
                    let matchpassword = MatchPassword::Private(privategamepassword.clone());
                    
                    //check if this game exists in the list of open matches
                    //if it does, send the port of the game back
                    //if not, do nothing
                    if let Some(portrequested) = main.openmatches.get( & matchpassword ){
                        
                        //send the port through the websocket
                        connectsucceeded = Main::send_connected_message_through_websocket(&mut newsocket, portrequested, &privategamepassword);
                        
                        //remove this match from the list of open matches
                        main.openmatches.remove(&matchpassword);
                    }
                }
            }


            //if it succesfully connected and sent that message to the client
            if connectsucceeded{
                break;
            }
        }
    };
    

    
    
    
}







use std::collections::HashMap;



struct Main{
    
    //the list of open matches
    openmatches: HashMap<MatchPassword, u16>,
    
    
    //the public password is the same for every public game
    publicpassword: String,
    
}



impl Main{
    
    fn new() -> Main{
        
        Main{
            openmatches: HashMap::new(),
            
            publicpassword: "password".to_string(),
        }
    }
    
    
    
    //true if it was sent successfully, false if it wasnt
    fn send_connected_message_through_websocket(socket: &mut tungstenite::WebSocket<std::net::TcpStream>, port: &u16, password: &String) -> bool{
        
        
        let connectedmessage = ConnectedToGame{
            gameport: *port,
            gamepassword: password.clone()
        };
        
        let connectedmessagestring = serde_json::to_string(&connectedmessage).unwrap();
        
        let message = Message::text(connectedmessagestring);
        
        
        
        if let Ok(_) =  socket.write_message(message){
            
            return true;
            
        }
        else{
            
            //send failed
            //hmmm
            //why?
            
            panic!("send failed. why? i should remove this panic also");
            
            return false;
        }
        
        
    }
    
    
    //create a game with a certain password
    //and return the port its open on
    fn create_game(password: String) -> u16{
        
        
        //get a random port that is open
        let randomopenport = get_available_port().unwrap();
        
        
        let command = format!("./new_websocket_game.sh");


        let randomopenportcopy = randomopenport.to_string();
        
        //let command = format!("ls");
        
        let handle = thread::spawn(|| {
    
            println!("creating websocket server");
            
            //run the websocket, with the password and the port it should run on
            let mut list_dir = Command::new(command);

            list_dir.arg(randomopenportcopy);
            list_dir.arg(password);
            list_dir.current_dir("..");

            list_dir.status().expect("process failed to execute");
        });
        
        
        //with the handle i can get when the game ends
        //and remove it from the list of open games
        //add this if it becomes a problem I guess
        
        
        randomopenport
    }
    
    
    
}




//the id of a match
#[derive(PartialEq, Eq, Hash)]
enum MatchPassword{
    
    //a private game with an associated password
    Private(String),
    
    //a public game
    Public,    
}





fn port_is_available(port: u16) -> bool {
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}

fn get_available_port() -> Option<u16> {
    (3000..4000).find(|port| port_is_available(*port))
}







use serde::{Serialize, Deserialize};



//a request for how the client wants to join a game
#[derive(Serialize, Deserialize, Debug)]
pub enum GameToConnectTo{
    
    joinpublicgame,
    joinprivategame(String),
    createprivategame,
}



//the message sent when a client is connected to a game on the server
//and the game is active
#[derive(Serialize, Deserialize)]
pub struct ConnectedToGame{
    
    //the port of the game
    gameport: u16,
    
    //the password of the game
    gamepassword: String,
}