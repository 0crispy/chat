use std::convert::Infallible;

use hyper::body::Buf;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use futures::{TryStreamExt as _, StreamExt, Stream};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct User {
    name: String,
    password: String,
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
            let name = body[0].strip_prefix("name=").unwrap();
            let password = body[1].strip_prefix("password=").unwrap();
            println!("name {}, password {}",name,password);
        }
        (&Method::GET, "/register") => {
            *response.body_mut() = Body::from(std::fs::read_to_string("register.html").unwrap());
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