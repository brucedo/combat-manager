
use axum::Router;
use axum::routing::get;
use log::{debug, error};
use tokio::sync::mpsc;

pub mod tracker;
pub mod http;
pub mod gamerunner;

use crate::gamerunner::dispatcher::Message;
use crate::http::metagame::Metagame;
// use crate::http::server::{new_game, get_example_char, add_new_character, change_game_state, get_state_demo};
use crate::http::server::start_server;
// use crate::http::renders::{index, create_game, game_view, no_session, new_session, add_npc, add_pc};
use crate::http::messaging::start_message_stream;
use crate::http::session::SessionMap;


#[tokio::main]
async fn main() {
    // Get logging enabled.
    env_logger::init();
    
    debug!("Beginning launch of Shadowrun Combat Manager");
    if let Ok(home_dir) = std::env::current_dir()
    {
        if let Some(home_dir_str) = home_dir.to_str()
        {
            debug!("Current directory: {}", home_dir_str);
        }
        else
        {
            error!("You've done it again, Kif: we don't have a directory.");
        }
    }
    
    let (runner_sender, runner_receiver) = mpsc::channel::<Message>(10);

    // let (mut main_sender, mut main_receiver) = mpsc::channel::<MainMessages>(2);

    // tokio::spawn(async move {launch_server(main_sender.clone()).await;});
    // tokio::spawn(start_server());
    tokio::spawn(async move {gamerunner::game_runner(runner_receiver).await;});

    let session_map = SessionMap::new();
    let game_state = Metagame::new(runner_sender);

    start_server().await;

}

#[derive(PartialEq)]
pub enum MainMessages
{
    Quit,
    Reload,
}