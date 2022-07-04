use log::debug;
use rocket::routes;
use tokio::sync::mpsc;

pub mod tracker;
pub mod http;
pub mod gamerunner;

use crate::{gamerunner::RequestMessage};

#[rocket::main]
async fn main() {
    // Get logging enabled.
    env_logger::init();
    
    debug!("Beginning launch of Shadowrun Combat Manager");

    let (runner_sender, runner_receiver) = mpsc::channel::<RequestMessage>(10);

    // let (mut main_sender, mut main_receiver) = mpsc::channel::<MainMessages>(2);

    // tokio::spawn(async move {launch_server(main_sender.clone()).await;});
    tokio::spawn(async move {gamerunner::game_runner(runner_receiver).await;});

    let _ = rocket::build()
        .manage(runner_sender)
        .mount("/", routes![http::new_game, http::get_example_char, http::add_new_character, http::change_game_state, http::get_state_demo])
        .launch()
        .await;
}

#[derive(PartialEq)]
pub enum MainMessages
{
    Quit,
    Reload,
}