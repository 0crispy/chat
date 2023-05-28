use std::convert::Infallible;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use futures::{StreamExt, SinkExt, TryStreamExt};
use hyper_tungstenite::{HyperWebsocket, WebSocketStream};
use hyper_tungstenite::tungstenite::Message as ClientMessage;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize,Serialize, Debug)]
struct Rooms {
    rooms:Vec<Room>
}
#[derive(Deserialize,Serialize,Debug)]
struct Room{
    name:String,
    info:String,
    id:String,
    messages:Vec<Message>,
}
#[derive(Deserialize,Serialize,Debug, Clone)]
struct Message{
    author:String,
    time:String,
    message:String,
}
impl Message{
    fn new(author:&str,message:&str) -> Self{
        Self{
            author: author.to_string(),
            time: chrono::Utc::now().to_string(),
            message: message.to_string(),
        }
    }
}
fn load_res(path:&str) -> String{
    std::fs::read_to_string("res/".to_string() + path).unwrap()
}
fn load_res_raw(path:&str) -> Vec<u8>{
    std::fs::read("res/".to_string() + path).unwrap()
}
fn get_room(room_id:&str) -> Option<Room>{
    match std::fs::read_to_string(format!("data/rooms/{}.json",room_id)){
        Ok(ok) => {
            Some(serde_json::from_str(&ok).unwrap())
        },
        Err(_) => {
            None
        },
    }
}
fn save_room(room:Room){
    std::fs::write(format!("data/rooms/{}.json",room.id),serde_json::to_string_pretty(&room).unwrap()).unwrap();
}
fn send_message(room_id:&str, author:&str, message:&str) -> Message{
    let mut room = get_room(room_id).unwrap();
    let msg = Message::new(author,message);
    room.messages.push(msg.clone());
    save_room(room);
    msg
}
fn get_messages(start_index:usize, room_id:&str) -> Vec<Message> {
    let room = get_room(room_id).unwrap();
    let messages = room.messages.clone();
    let len = (messages.len()-start_index).min(10);
    messages[start_index..(start_index + len)].to_vec()
}
async fn serve_websocket(websocket: HyperWebsocket) -> Result<(), hyper_tungstenite::tungstenite::Error> {
    let mut websocket = websocket.await?;
    while let Some(message) = websocket.next().await {
        match message?{
            ClientMessage::Text(text) => {
                let prefix = text.split(":").collect::<Vec<_>>()[0];
                let msg = text.strip_prefix(&(prefix.to_string() + ":")).unwrap();
                match prefix{
                    "pls" => {
                        println!("Shut up man");
                        #[derive(Deserialize)]
                        struct Pls{
                            id:String,
                            count:usize,
                        }
                        let pls = serde_json::from_str::<Pls>(msg).unwrap();
                        //send messages
                        let messages = get_messages(pls.count,&pls.id);
                        for message in messages{
                            println!("fuck {}", pls.id);
                            let send_msg = ("new_message:".to_string() + &serde_json::to_string(&message).unwrap()).to_string();
                            websocket.send(ClientMessage::Text(send_msg)).await?;
                        }
                    },
                    "msg" => {
                        #[derive(Deserialize)]
                        struct BasicMessage{
                            author:String,
                            id:String,
                            msg:String,
                        }
                        let basic_msg = serde_json::from_str::<BasicMessage>(msg).unwrap();
                        let sent_msg = send_message(&basic_msg.id, &basic_msg.author, &basic_msg.msg);
                        let send_msg = ("new_message:".to_string() + &serde_json::to_string(&sent_msg).unwrap()).to_string();
                        websocket.send(ClientMessage::Text(send_msg)).await?;
                    },
                    _ => {}
                }
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
            *response.body_mut() = Body::from(load_res("hello.html"));
        },
        (&Method::POST, "/getRooms") => {
            let rooms:Vec<Room> = std::fs::read_dir("data/rooms")
                .unwrap()
                .into_iter()
                .map(|x|{
                    let path = x.unwrap().path();
                    let path_str = path.to_str().unwrap();
                    let room_id = path_str.strip_prefix("data/rooms/").unwrap().strip_suffix(".json").unwrap();
                    get_room(room_id).unwrap()
                })
                .collect();
            #[derive(Serialize)]
            struct BasicRoom{
                name:String,
                info:String,
                id:String,
            }
            #[derive(Serialize)]
            struct OutputMessage{
                rooms:Vec<BasicRoom>,
            }
            let output_rooms = rooms.iter().map(|x|BasicRoom{name:x.name.clone(),info:x.info.clone(),id:x.id.clone()}).collect::<Vec<_>>();
            *response.body_mut() = Body::from(serde_json::to_string(&OutputMessage{
                rooms:output_rooms
            }).unwrap());

        },
        (&Method::POST, "/chat") => {
            *response.body_mut() = Body::from(load_res("chat.html"));
        },
        (&Method::GET, "/favicon.ico") => {
            *response.body_mut() = Body::from(load_res_raw("favicon.ico"));
        },
        (&Method::GET, "/room") => {
            let uri = req.uri().to_string();
            let room_id = uri.strip_prefix("/room?id=").unwrap();
            if get_room(room_id).is_some(){
                let mut body_str = load_res("room.html");
                body_str = body_str.replace("ROOM_ID", room_id);
                *response.body_mut() = Body::from(body_str);
            }
            else{
                *response.body_mut() = Body::from(load_res("room_not_found.html"));
            }
        },
        (_,_path) => {
            *response.status_mut() = StatusCode::NOT_FOUND;
            *response.body_mut() = Body::from(load_res("404.html"));
        },
    };

    Ok(response)}
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    let addr:std::net::SocketAddr = ([127, 0, 0, 1], 3000).into();
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("listening on http://{}", addr);
    let mut http = hyper::server::conn::Http::new();
    http.http1_only(true);
    http.http1_keep_alive(true);

    loop {
        let (stream, _) = listener.accept().await?;
        let connection = http
            .serve_connection(stream, hyper::service::service_fn(hello))
            .with_upgrades();
        tokio::spawn(async move {
            if let Err(err) = connection.await {
                println!("Error serving HTTP connection: {:?}", err);
            }
        });
    }
}