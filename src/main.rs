
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


    let (runner_sender, runner_receiver) = mpsc::channel::<Message>(10);

    // let (mut main_sender, mut main_receiver) = mpsc::channel::<MainMessages>(2);

    // tokio::spawn(async move {launch_server(main_sender.clone()).await;});
    // tokio::spawn(start_server());
    tokio::spawn(async move {gamerunner::game_runner(runner_receiver).await;});

    let session_map = SessionMap::new();
    let game_state = Metagame::new(runner_sender);

    start_server().await;

}



fn load_templates(template_dirs: Vec<PathBuf>, handlebars: &mut handlebars::Handlebars) -> Result<(), Error>
{
    // let mut handlebars = handlebars::Handlebars::new();

    for template_dir in template_dirs
    {

        debug!("Loading handlebar template files found in {}", template_dir.display());

        let templates = template_dir.read_dir()?
            .filter(|rd| rd.is_ok())
            .map(|rd| rd.unwrap())
            .filter(|de| de.path().is_file()  && de.path().extension().is_some() && de.path().extension().unwrap() == "hbs")
            .collect::<Vec<DirEntry>>();
        
        for entry in templates
        {
            debug!("Loading template {}", entry.file_name().to_str().unwrap());
            match (entry.file_name().into_string(), fs::read_to_string(entry.path()))
            {
                (Ok(fq_name), Ok(contents)) => {
                    let name = match fq_name.find(".")
                    {
                        Some(prefix) => &fq_name[0..prefix],
                        None => &fq_name
                    };                

                    if let Err(template_err) = handlebars.register_template_string(name, contents)
                    {
                        error!("An error occurred while loading template {}, reason: {}", fq_name, template_err.reason())
                    }
                }
                (Err(e), _) => {

                }
                (_, Err(e)) => { 
                    return Err(e);
                }
            };
                
        }

    }

    Ok(())
}

#[derive(PartialEq)]
pub enum MainMessages
{
    Quit,
    Reload,
}