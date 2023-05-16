use std::collections::HashMap;
use std::convert::Infallible;
use std::fs::OpenOptions;
use std::io::Write;

use hyper::body::Buf;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use futures::{TryStreamExt as _, StreamExt, Stream};
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

async fn hello(req: Request<Body>) -> Result<Response<Body>, Infallible> {
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
        (&Method::POST, "/register") => {
            let body = hyper::body::to_bytes(req.into_body()).await.unwrap().iter()
                .cloned()
                .collect::<Vec<u8>>();
            let body = String::from_utf8(body).unwrap();
            let body = body.split('&').collect::<Vec<&str>>();

            let username = body[0].strip_prefix("name=").unwrap();
            let password = body[1].strip_prefix("hashedPassword=").unwrap();
            let mut users:Users = serde_json::from_str(&std::fs::read_to_string("data/users.json").unwrap()).unwrap();
            if users.get_user(username).is_none(){
                users.add_user(username, password);
            }
            else{
                //user already exists
            }
            std::fs::write("data/users.json",serde_json::to_string_pretty(&users).unwrap()).unwrap();
        },
        (&Method::GET, "/script.js") => {            
            *response.body_mut() = Body::from(std::fs::read_to_string("script.js").unwrap());
        },
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
            *response.body_mut() = Body::from(std::fs::read_to_string("404.html").unwrap());
        },
    };

    Ok(response)
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