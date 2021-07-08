use hyper::service::{make_service_fn, service_fn};
use hyper::{Request, Body, Response, StatusCode, Method, Server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = ([127, 0, 0, 1], 3000).into();
    let server = Server::bind(&addr).serve(make_service_fn(|_| async {
        Ok::<_, hyper::Error>(service_fn(handle_req))
    }));
    println!("Listening on http://{}", addr);
    server.await?;
    Ok(())
}

async fn handle_req(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/item/add") => {
            Ok(Response::new("add succeed".into()))
        }
        (&Method::POST, "/item/remove") => {
            Ok(Response::new("remove succeed".into()))
        }
        (&Method::GET, "/item/query") => {
            Ok(Response::new("query succeed".into()))
        }
        _ => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap())
    }
}
