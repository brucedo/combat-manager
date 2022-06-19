use log::debug;

pub mod game;
pub mod dispatcher;

use crate::dispatcher::dispatcher::launch_server;

#[tokio::main]
async fn main() {
    // Get logging enabled.
    env_logger::init();
    
    debug!("Beginning launch of Shadowrun Combat Manager");

    tokio::spawn(async {launch_server().await;});
}
