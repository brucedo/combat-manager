use std::collections::HashMap;
use std::sync::Arc;

use log::{debug, error};
use rocket::fs::{FileServer, relative};
use rocket::routes;
use rocket_dyn_templates::Template;
use tokio::sync::mpsc;
use uuid::Uuid;

pub mod tracker;
pub mod http;
pub mod gamerunner;

use crate::gamerunner::Message;
use crate::http::server::{new_game, get_example_char, add_new_character, change_game_state, get_state_demo};
use crate::http::renders::{index, create_game, gm_view};
use crate::http::session::{SessionMap, Session};

#[rocket::main]
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
    tokio::spawn(async move {gamerunner::game_runner(runner_receiver).await;});

    let mut session_map = SessionMap { sessions: Arc::new(HashMap::<Uuid, Session>::new()) };

    let _ = rocket::build()
        .manage(runner_sender)
        .manage(session_map)
        .mount("/res", FileServer::from(relative!("resources/static")))
        .mount("/", routes![index, create_game, gm_view])
        .mount("/api", routes![new_game, get_example_char, add_new_character, change_game_state, get_state_demo])
        .attach(Template::fairing())
        .launch()
        .await;
}

#[derive(PartialEq)]
pub enum MainMessages
{
    Quit,
    Reload,
}