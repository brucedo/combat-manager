use log::debug;
use poem::{Route, get, handler, listener::TcpListener, Server, Request};
use tokio::sync::mpsc::Sender;

use crate::MainMessages;


pub async fn launch_server(main_sender: Sender<MainMessages>)
{
    debug!("Server launched - taking requests.");
    // let temp = main_sender.clone();
    let made_api_handler = poem::endpoint::make(move |req| { 
        let moved = main_sender.clone();
        async move {api_handler(moved, req).await;}
    });
    let routes = Route::new().at("/api", get(made_api_handler));
    Server::new(TcpListener::bind("localhost:8080")).run(routes).await;
}

#[handler]
pub fn bootstrap() -> &'static str
{
    
    return "A basic handler.";
}

pub async fn api_handler(sender: Sender<MainMessages>, req: Request)
{

}