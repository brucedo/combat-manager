
use std::fs::{self, DirEntry};
use std::io::Error;
use std::path::PathBuf;
use std::process::exit;

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
use crate::http::server::{start_server};
// use crate::http::renders::{index, create_game, game_view, no_session, new_session, add_npc, add_pc};
use crate::http::messaging::start_message_stream;
use crate::http::session::SessionMap;


#[tokio::main]
async fn main() {
    // Get logging enabled.
    env_logger::init();
    
    debug!("Beginning launch of Shadowrun Combat Manager");

    let config = Configuration {
        static_path: PathBuf::from("resources/static"),
        template_path: PathBuf::from("resources/templates"),
        bind_addr: String::from("0.0.0.0"),
        bind_port: String::from("8080")
    };


    let (runner_sender, runner_receiver) = mpsc::channel::<Message>(10);

    // let (mut main_sender, mut main_receiver) = mpsc::channel::<MainMessages>(2);

    // tokio::spawn(async move {launch_server(main_sender.clone()).await;});
    // tokio::spawn(start_server());
    tokio::spawn(async move {gamerunner::game_runner(runner_receiver).await;});

    let session_map = SessionMap::new();
    let game_state = Metagame::new(runner_sender);

    start_server(&config).await;

}

pub struct Configuration
{
    pub static_path: PathBuf,
    pub template_path: PathBuf,
    pub bind_addr: String,
    pub bind_port: String,
}

// #[derive(PartialEq)]
// pub enum MainMessages
// {
//     Quit,
//     Reload,
// }