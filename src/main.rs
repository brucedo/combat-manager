
use std::fs::{self, DirEntry};
use std::io::Error;
use std::path::PathBuf;
use std::process::exit;

use axum::Router;
use axum::routing::get;
use log::{debug, error, logger};
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
    let application_root = match std::env::current_dir()
    {
        Ok(application_root) => application_root,
        Err(_) => {error!("Application has not been started in a valid filesystem context"); exit(-1);}
    };

    let mut templates = application_root.clone();
    let mut errors = application_root.clone();

    templates.push("resources");
    templates.push("templates");

    errors.push("resources");
    errors.push("templates");
    errors.push("error_pages");
    let template_dirs = vec![templates, errors];
    let mut templates = handlebars::Handlebars::new();
    match load_templates(template_dirs, &mut templates)
    {
        Ok(templates) => templates,
        Err(_) => {error!("Unable to load the application templates."); exit(-1);}
    };

    
    let (runner_sender, runner_receiver) = mpsc::channel::<Message>(10);

    // let (mut main_sender, mut main_receiver) = mpsc::channel::<MainMessages>(2);

    // tokio::spawn(async move {launch_server(main_sender.clone()).await;});
    // tokio::spawn(start_server());
    tokio::spawn(async move {gamerunner::game_runner(runner_receiver).await;});

    let session_map = SessionMap::new();
    let game_state = Metagame::new(runner_sender);

    start_server(templates).await;

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