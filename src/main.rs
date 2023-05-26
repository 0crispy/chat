use std::collections::HashMap;
use std::convert::Infallible;
use std::fs::OpenOptions;
use std::io::Write;

use hyper::body::Buf;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use futures::{TryStreamExt as _, StreamExt, Stream, SinkExt};
use hyper_tungstenite::HyperWebsocket;
use hyper_tungstenite::tungstenite::Message as ClientMessage;
use serde::{Deserialize, Serialize};

#[derive(Deserialize,Serialize, Debug)]
struct Users {
    users:Vec<User>
}
impl Users{
    fn get_user(&self,username:&str) -> Option<&User>{
        self.users.iter().find(|x|x.username==username)
    }
    fn add_user(&mut self, username:&str,password:&str){
        self.users.push(User {username:username.to_string(),password:password.to_string() });
    }

}
#[derive(Deserialize,Serialize,Debug)]
struct User{
    username:String,
    password:String,
}
#[derive(Deserialize,Serialize, Debug)]
struct Rooms {
    rooms:Vec<Room>
}
impl Rooms{
    fn get_room(&self,name:&str) -> Option<&Room>{
        self.rooms.iter().find(|x|x.name==name)
    }
}
#[derive(Deserialize,Serialize,Debug)]
struct Room{
    name:String,
    info:String,
    id:String,
    messages:Vec<Message>,
}
#[derive(Deserialize,Serialize,Debug)]
struct Message{
    author:String,
    time:String,
    message:String,
}
async fn serve_websocket(websocket: HyperWebsocket) -> Result<(), hyper_tungstenite::tungstenite::Error> {
    let mut websocket = websocket.await?;
    while let Some(message) = websocket.next().await {
        match message?{
            ClientMessage::Text(text) => {
                println!("{}" ,text);
                websocket.send(ClientMessage::Text("whats up".to_string())).await?;
            },
            _ =>{}
        }
    }

    Ok(())
}
async fn hello(mut req: Request<Body>) -> Result<Response<Body>, hyper_tungstenite::tungstenite::Error> {
    if hyper_tungstenite::is_upgrade_request(&req) {
        let (response, websocket) = hyper_tungstenite::upgrade(&mut req, None)?;

        // Spawn a task to handle the websocket connection.
        tokio::spawn(async move {
            if let Err(e) = serve_websocket(websocket).await {
                eprintln!("Error in websocket connection: {}", e);
            }
        });

        // Return the response so the spawned future can continue.
        Ok(response)
    } else {

    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            *response.body_mut() = Body::from(std::fs::read_to_string("hello.html").unwrap());
        },        
        (&Method::POST, "/login") => {    
            let body = hyper::body::to_bytes(req.into_body()).await.unwrap().iter()
                .cloned()
                .collect::<Vec<u8>>();
            let body = String::from_utf8(body).unwrap();
            let body = body.split('&').collect::<Vec<&str>>();

            let username = body[0].strip_prefix("name=").unwrap();
            let password = body[1].strip_prefix("hashedPassword=").unwrap();
            
            let users:Users = serde_json::from_str(&std::fs::read_to_string("data/users.json").unwrap()).unwrap();
            if let Some(user) = users.get_user(username){
                if user.password == password{
                    //nice
                }
                else{
                    //incorrect password!
                }
            }
            else{
                //no such user!
            }
            
        }
        (&Method::GET, "/register") => {
            *response.body_mut() = Body::from(std::fs::read_to_string("register.html").unwrap());
        },
        (&Method::POST, "/tryLogin") => {
            let body = hyper::body::to_bytes(req.into_body()).await.unwrap().iter()
                .cloned()
                .collect::<Vec<u8>>();
            let body = String::from_utf8(body).unwrap();
            let user:User = serde_json::from_str(&body).unwrap();            
            
            let mut output_message = "ok";
            let users:Users = serde_json::from_str(&std::fs::read_to_string("data/users.json").unwrap()).unwrap();
            if let Some(found_user) = users.get_user(&user.username){
                if found_user.password == user.password{
                    //nice
                }
                else{
                    output_message = "wrong_password";
                }
            }
            else{
                output_message = "no_user";
            }
            #[derive(Serialize)]
            struct OutputMessage<'a>{
                message:&'a str,
            }
            *response.body_mut() = Body::from(serde_json::to_string(&OutputMessage{message:output_message}).unwrap());
        },
        (&Method::POST, "/tryRegister") => {
            let body = hyper::body::to_bytes(req.into_body()).await.unwrap().iter()
                .cloned()
                .collect::<Vec<u8>>();
            let body = String::from_utf8(body).unwrap();
            let user:User = serde_json::from_str(&body).unwrap();            
            let mut users:Users = serde_json::from_str(&std::fs::read_to_string("data/users.json").unwrap()).unwrap();
            
            let mut output_message = "ok";
            if users.get_user(&user.username).is_none(){
                users.add_user(&user.username, &user.password);
                std::fs::write("data/users.json",serde_json::to_string_pretty(&users).unwrap()).unwrap();
            }
            else{
                output_message = "taken";
            }
            #[derive(Serialize)]
            struct OutputMessage<'a>{
                message:&'a str,
            }
            *response.body_mut() = Body::from(serde_json::to_string(&OutputMessage{message:output_message}).unwrap());
        },
        (&Method::POST, "/getRooms") => {
            let rooms:Rooms = serde_json::from_str(&std::fs::read_to_string("data/rooms.json").unwrap()).unwrap();
            #[derive(Serialize)]
            struct Room{
                name:String,
                info:String,
                id:String,
            }
            #[derive(Serialize)]
            struct OutputMessage{
                rooms:Vec<Room>,
            }
            let output_rooms = rooms.rooms.iter().map(|x|Room{name:x.name.clone(),info:x.info.clone(),id:x.id.clone()}).collect::<Vec<_>>();
            *response.body_mut() = Body::from(serde_json::to_string(&OutputMessage{
                rooms:output_rooms
            }).unwrap());

        },
        /*(&Method::POST, "/joinRoom") => {
            let body = hyper::body::to_bytes(req.into_body()).await.unwrap().iter()
                .cloned()
                .collect::<Vec<u8>>();
            let body = String::from_utf8(body).unwrap();
            #[derive(Deserialize)]
            struct Room{
                name:String,password:String
            }
            let room:Room = serde_json::from_str(&body).unwrap();
            let (room_name,room_password):(&str,&str) = (&room.name,&room.password);
            let rooms:Rooms = serde_json::from_str(&std::fs::read_to_string("data/rooms.json").unwrap()).unwrap();
            let mut output_message = "ok";
            if let Some(room) = rooms.get_room(room_name){
                if room.password == room_password{
                    //good
                }
                else{
                    //incorrect password
                    output_message = "wrong_password";
                }
            }
            else{
                //no such room?? unexpected error.
                output_message = "unexpected";
            }
            #[derive(Serialize)]
            struct OutputMessage<'a>{
                message:&'a str,
            }
            *response.body_mut() = Body::from(serde_json::to_string(&OutputMessage{message:output_message}).unwrap());
        },*/
        (&Method::POST, "/chat") => {
            *response.body_mut() = Body::from(std::fs::read_to_string("chat.html").unwrap());
        },
        (&Method::GET, "/favicon.ico") => {
            *response.body_mut() = Body::from(std::fs::read("favicon.ico").unwrap());
        },
        (&Method::GET, "/room") => {
            let uri = req.uri().to_string();
            let room_id = uri.strip_prefix("/room?id=").unwrap();
            *response.body_mut() = Body::from(std::fs::read_to_string("room.html").unwrap());
        },
        (_,_path) => {
            *response.status_mut() = StatusCode::NOT_FOUND;
            *response.body_mut() = Body::from(std::fs::read_to_string("404.html").unwrap());
        },
    };

    Ok(response)}
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // For every connection, we must make a `Service` to handle all
    // incoming HTTP requests on said connection.
    let make_svc = make_service_fn(|_conn| {
        // This is the `Service` that will handle the connection.
        // `service_fn` is a helper to convert a function that
        // returns a Response into a `Service`.
        async { Ok::<_, Infallible>(service_fn(hello)) }
    });

    let addr = ([127, 0, 0, 1], 3000).into();

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}