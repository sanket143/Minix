use env_logger;
use crate::config;

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use tokio::runtime;
use warp::{ws::Message, Filter, Rejection};

mod handler;
mod ws;

type Result<T> = std::result::Result<T, Rejection>;
type Clients = Arc<RwLock<HashMap<String, Client>>>;

#[derive(Debug, Clone)]
pub struct Client {
  pub username: String,
  pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

pub fn init() {
  tauri::spawn(move || {
    println!("websocket: {:?}", std::thread::current().id());
    runtime::Runtime::new().unwrap().block_on(async {
      serve().await;
    });
  });
}

pub async fn serve() {
  env_logger::init();
  let clients: Clients = Arc::new(RwLock::new(HashMap::new()));
  let health_route = warp::path!("health").and_then(handler::health_handler);

  let register = warp::path("register");
  let register_routes = register
    .and(warp::post())
    .and(warp::body::form())
    .and(with_clients(clients.clone()))
    .and_then(handler::register_handler)
    .or(
      register
        .and(warp::delete())
        .and(warp::path::param())
        .and(with_clients(clients.clone()))
        .and_then(handler::unregister_handler),
    );

  let publish = warp::path!("publish")
    .and(warp::body::json())
    .and(with_clients(clients.clone()))
    .and_then(handler::publish_handler);

  let ws_route = warp::path("ws")
    .and(warp::ws())
    .and(warp::path::param())
    .and(with_clients(clients.clone()))
    .and_then(handler::ws_handler);

  let routes = health_route
    .or(ws_route)
    .or(register_routes)
    .or(publish)
    .with(
      warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST", "DELETE", "OPTION"]),
    );

  warp::serve(routes)
    .run(([0, 0, 0, 0], config::WS_SERVER_PORT))
    .await;
}

fn with_clients(clients: Clients) -> impl Filter<Extract = (Clients,), Error = Infallible> + Clone {
  warp::any().map(move || clients.clone())
}
