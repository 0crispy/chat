use hyper::{Body, Request, Response, Method, StatusCode};
use futures::{StreamExt, SinkExt};
use hyper_tungstenite::{HyperWebsocket};
use hyper_tungstenite::tungstenite::Message as ClientMessage;
use serde::{Deserialize, Serialize};

#[derive(Deserialize,Serialize, Debug)]
struct Users {
    users:Vec<User>
}
impl Users{
    fn load() -> Users{
        serde_json::from_str(std::fs::read_to_string("data/users.json").unwrap().as_str()).unwrap()
    }
    fn save(&self){
        std::fs::write("data/users.json", serde_json::to_string_pretty(&self).unwrap()).unwrap();
    }
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
            time: chrono::Utc::now().naive_local().to_string(),
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
fn get_room_by_name(room_name:&str) -> Option<Room>{
    for file in std::fs::read_dir("data/rooms").unwrap() {
        let room_path = file.unwrap().path();
        let room_path_str = room_path.to_str().unwrap();
        let room_str = std::fs::read_to_string(&room_path_str).unwrap();
        let room = serde_json::from_str::<Room>(&room_str).unwrap();
        if room.name == room_name{
            return Some(room);
        }
    }
    None
}
fn save_room(room:Room){
    std::fs::write(format!("data/rooms/{}.json",room.id),serde_json::to_string_pretty(&room).unwrap()).unwrap();
}
fn get_new_room_id() -> usize{
    let mut id = 0;
    for file in std::fs::read_dir("data/rooms").unwrap() {
        let room_path = file.unwrap().path();
        let room_path_str = room_path.to_str().unwrap();
        let room_id = room_path_str.strip_prefix("data/rooms/").unwrap().strip_suffix(".json").unwrap().parse::<usize>().unwrap();
        id = id.max(room_id);
    }
    id+1
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
    let len = messages.len()-start_index;
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
                        #[derive(Deserialize)]
                        struct Pls{
                            id:String,
                            count:usize,
                        }
                        let pls = serde_json::from_str::<Pls>(msg).unwrap();
                        //send messages
                        let messages = get_messages(pls.count,&pls.id);
                        for message in messages{
                            let send_msg = ("new_msg:".to_string() + &serde_json::to_string(&message).unwrap()).to_string();
                            websocket.send(ClientMessage::Text(send_msg)).await?;
                        }
                    },
                    "msg" => {
                        #[derive(Deserialize)]
                        struct BasicMessage{
                            author:String,
                            password:String,
                            id:String,//room id
                            msg:String,
                        }
                        let basic_msg = serde_json::from_str::<BasicMessage>(msg).unwrap();
                        let users = Users::load();
                        let mut ok = false;
                        if let Some(user) = users.get_user(&basic_msg.author){
                            if user.password == basic_msg.password{
                                ok = true;
                            }
                        }
                        if ok{
                            let sent_msg = send_message(&basic_msg.id, &basic_msg.author, &basic_msg.msg);
                            let send_msg = ("new_msg:".to_string() + &serde_json::to_string(&sent_msg).unwrap()).to_string();
                            websocket.send(ClientMessage::Text(send_msg)).await?;
                        }
                        else{
                            websocket.send(ClientMessage::Text("disconnect:true".to_string())).await?;
                        }
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
            *response.body_mut() = Body::from(load_res("login.html"));
        },
        (&Method::GET, "/home") => {
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
                msg_count:usize,
            }
            #[derive(Serialize)]
            struct OutputMessage{
                rooms:Vec<BasicRoom>,
            }
            let mut output_rooms = rooms.iter().map(|x|BasicRoom{
                name:x.name.clone(),
                info:x.info.clone(),
                id:x.id.clone(),
                msg_count: x.messages.len()
            }).collect::<Vec<_>>();
            output_rooms.sort_by(|x,y|y.msg_count.cmp(&x.msg_count));
            *response.body_mut() = Body::from(serde_json::to_string(&OutputMessage{
                rooms:output_rooms
            }).unwrap());

        },
        (&Method::POST, "/chat") => {
            *response.body_mut() = Body::from(load_res("chat.html"));
        },
        (&Method::GET, "/favicon.ico") => {
            *response.body_mut() = Body::from(load_res_raw("mushroom.ico"));
        },
        (&Method::GET, "/room") => {
            let uri = req.uri().to_string();
            let room_id = 
                if let Some(id) = uri.strip_prefix("/room?id="){
                    Some(id)
                }
                else{
                    None
                };
            let mut is_ok = false;
            if let Some(room_id) = room_id{
                if get_room(room_id).is_some(){
                    let mut body_str = load_res("room.html");
                    body_str = body_str.replace("ROOM_ID", room_id);
                    *response.body_mut() = Body::from(body_str);
                    is_ok = true;
                }
            }
            if !is_ok{
                *response.body_mut() = Body::from(load_res("room_not_found.html"));
            }
        },
        (&Method::POST, "/login") => {
            let body = hyper::body::to_bytes(req.into_body()).await.unwrap().iter()
                .cloned()
                .collect::<Vec<u8>>();
            let body = String::from_utf8(body).unwrap();
            let user = serde_json::from_str::<User>(body.as_str()).unwrap();
            let users = Users::load();
            #[derive(Serialize)]
            struct OutputMessage{
                ok:bool,
                err:String,
            }
            let mut output = OutputMessage{ok:true,err:String::new()};
            if let Some(current) = users.get_user(&user.username){
                if current.password != user.password{
                    output.ok = false;
                    output.err = "wrong_password".to_string();
                }
            }
            else{
                output.ok = false;
                output.err = "no_acc".to_string();
            }
            *response.body_mut() = Body::from(serde_json::to_string(&output).unwrap());
        },
        (&Method::POST, "/register") => {
            let body = hyper::body::to_bytes(req.into_body()).await.unwrap().iter()
                .cloned()
                .collect::<Vec<u8>>();
            let body = String::from_utf8(body).unwrap();
            let user = serde_json::from_str::<User>(body.as_str()).unwrap();
            let mut users = Users::load();
            #[derive(Serialize)]
            struct OutputMessage{
                ok:bool,
                err:String,
            }
            let mut output = OutputMessage{ok:true,err:String::new()};
            if users.get_user(&user.username).is_some(){
                output.ok = false;
                output.err = "acc_exists".to_string();
            }
            else{
                users.add_user(&user.username, &user.password);
                users.save();
            }
            *response.body_mut() = Body::from(serde_json::to_string(&output).unwrap());
        },
        (&Method::POST, "/createRoom") => {
            let body = hyper::body::to_bytes(req.into_body()).await.unwrap().iter()
                .cloned()
                .collect::<Vec<u8>>();
            
            #[derive(Deserialize)]
            struct CreateRoom{
                name:String,
                info:String,
            }
            #[derive(Serialize)]
            struct Output{
                ok:bool,
                id:String,
            }
            let body = String::from_utf8(body).unwrap();
            let create_room = serde_json::from_str::<CreateRoom>(&body).unwrap();
            if get_room_by_name(&create_room.name).is_none(){
                let room_id =get_new_room_id().to_string(); 
                let new_room = Room{
                    name: create_room.name,
                    info: create_room.info,
                    id: room_id.clone(),
                    messages: Vec::new(),
                };
                save_room(new_room);
                *response.body_mut() = Body::from(serde_json::to_string(&Output{
                    ok: true,
                    id: room_id,
                }).unwrap());
            }
            else{
                *response.body_mut() = Body::from(serde_json::to_string(&Output{
                    ok: false,
                    id: String::new(),
                }).unwrap());
            }


        },
        (&Method::GET, "/register") => {
            *response.body_mut() = Body::from(load_res("register.html"));
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