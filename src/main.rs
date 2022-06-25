use log::debug;
use tokio::sync::mpsc;

pub mod tracker;
pub mod dispatcher;
pub mod gamerunner;

use crate::{dispatcher::launch_server, gamerunner::RequestMessage};

#[tokio::main]
async fn main() {
    // Get logging enabled.
    env_logger::init();
    
    debug!("Beginning launch of Shadowrun Combat Manager");

    let (mut runner_sender, mut runner_receiver) = mpsc::channel::<RequestMessage>(10);

    let (mut main_sender, mut main_receiver) = mpsc::channel::<MainMessages>(2);

    tokio::spawn(async move {launch_server(main_sender.clone()).await;});
    tokio::spawn(async move {gamerunner::game_runner(runner_receiver).await;});

    while let Some(message) = main_receiver.recv().await
    {
        if message == MainMessages::Quit
        {
            break;
        }
    }
}

#[derive(PartialEq)]
pub enum MainMessages
{
    Quit,
    Reload,
}