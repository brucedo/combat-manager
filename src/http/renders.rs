
use std::{process::exit, path::PathBuf, fs::{DirEntry, self}, collections::{HashMap, HashSet}, ffi::{OsString, OsStr}, io::ErrorKind};
use std::io::Error;
use axum::{response::Response, body::Bytes, extract::Path};
use axum_macros::debug_handler;
use log::{debug, error};
use uuid::Uuid;
use handlebars::{Handlebars, template};
use tokio::sync::{oneshot::channel, mpsc::Sender};

use crate::{gamerunner::dispatcher::{Message, Request, Outcome}, http::{session::NewSessionOutcome, models::NewGame}, tracker::character::Character, Configuration};

use super::{models::{GameSummary, GMView, IndexModel, PlayerView, SimpleCharacterView, NewCharacter}, session::Session, metagame::Metagame, modelview::StaticView, statics::Statics};


pub fn initialize_renders(config: &Configuration) -> (Handlebars<'static>, Statics)
{

    let mut templates = handlebars::Handlebars::new();
    load_templates(&config.template_path, &mut templates);    
    
    match Statics::with_root(&config.static_path)
    {
        Ok(statics) => (templates, statics),
        Err(super::statics::Error::CouldNotLoadManifestFile) => {
            error!("Could not load manifest file from {}", &config.static_path.display());
            panic!("Could not load manifest file.");
        },
        Err(super::statics::Error::FilePathNotStringable) => {
            error!("Some component of the path could not be rendered into string form.");
            panic!("Some component of the path could not be rendered into string form.");
        }
        Err(super::statics::Error::ManifestNotExists) => {
            error!("The manifest.toml file was not found in the expected location {}/static/manifest.toml.", &config.static_path.display());
            panic!("The manifest.toml file was not found.");
        }
        Err(super::statics::Error::StaticDirNotExists) => {
            error!("The directory {}/static does not exist", &config.static_path.display());
            panic!("The static directory does not exist.")
        }
        Err(super::statics::Error::CouldNotLoadStaticFile(f)) => {
            error!("A static file could not be loaded: {}", f);
            panic!("A static file could not be loaded.");
        }
    }
    
}

fn load_templates(root_path: &PathBuf, handlebars: &mut handlebars::Handlebars)
{
    let mut valid_extensions = HashSet::new();
    valid_extensions.insert(OsString::from("hbs"));

    let templates_to_load = recursive_file_filter(root_path, &valid_extensions);

    for template_path in templates_to_load
    {
        match read_text_file(&template_path)
        {
            Ok((filename, contents)) => 
            {
                let name = match filename.find(".") {
                    Some(position) => &filename[0..position],
                    None => filename.as_str()
                };

                debug!("Registering template {}", name);

                if let Err(_) = handlebars.register_template_string(name, contents) {
                    error!("The Handlebars service could not read template {}, please check the formatting and try again.", filename);
                }
            },
            Err(_) =>
            {
                error!("We were unable to load all of the template files correctly.  The application may not perform as expected.");
            }
        }
    }

}

fn recursive_file_filter(root_path: &PathBuf, extension_filter: &HashSet<OsString>) -> Vec<PathBuf>
{
    let mut passing_files = Vec::<PathBuf>::new();
    let mut directory_stack = vec![root_path.clone()];

    while directory_stack.len() > 0
    {
        let path = directory_stack.pop().unwrap();

        match path.read_dir()
        {
            Ok(dir_entries) => {
                for result in dir_entries
                {
                    match result 
                    {
                        Ok(dir_entry) => {
                            let fs_obj_path = dir_entry.path();

                            if fs_obj_path.is_dir() { 
                                directory_stack.push(fs_obj_path)
                            }
                            else if fs_obj_path.is_file() && fs_obj_path.extension().is_some() && extension_filter.contains(fs_obj_path.extension().unwrap()) { 
                                passing_files.push(fs_obj_path) 
                            }
                        },
                        Err(_) => {error!("Encountered an IO error while attempting to read an object in directory {}", path.display())}
                    }
                }
            },
            Err(_) => {
                error!("Encountered an IO error while attempting to read the contents of path {}", path.display())
            }
        }
    };

    passing_files
}

fn read_text_file(file_path: &PathBuf) -> Result<(String, String), Error>
{
    match (file_path.file_name(), fs::read_to_string(file_path))
    {
        (Some(filename), Ok(contents)) => {
            if let Ok(string_name) = filename.to_os_string().into_string()
            {
                Ok((string_name, contents))
            }
            else
            {
                Err(Error::new(ErrorKind::Unsupported, "The filename could not be converted into a standard UTF-8 string."))
            }
        }, 
        (_, _) => {
            Err(Error::new(ErrorKind::InvalidData, "Something went pretty catastrophically wrong with a file read.  Are you sure you're cut out for this?"))
        }
    }
}

#[debug_handler]
pub async fn index() -> Response<axum::body::Empty<Bytes>>
{
    
    Response::builder()
        .extension(StaticView{ view: String::from("index.html") })
        .body(axum::body::Empty::<Bytes>::new()).unwrap()
}

pub async fn static_resources(resource: Path<String>) -> Response<axum::body::Empty<Bytes>>
{
    Response::builder()
        .extension(StaticView{view: resource.0})
        .body(axum::body::Empty::<Bytes>::new()).unwrap()
}

#[debug_handler]
pub async fn display_registration_form() -> Response<axum::body::Empty<Bytes>>
{
    
    Response::builder()
        .extension(StaticView{view: String::from("register.html")})
        .body(axum::body::Empty::<Bytes>::new()).unwrap()
}

// #[post("/game", data = "<new_game>")]
// pub async fn create_game(state: &State<Metagame<'_>>, session: Session, new_game: Form<NewGame<'_>>) -> Result<Redirect, Error>
// {
//     let my_sender = state.game_runner_pipe.clone();

//     let response = send_and_recv(Uuid::new_v4(), Request::New, my_sender).await?;

//     match response
//     {
//         Outcome::Created(game_id) =>
//         {   
            
//             state.new_game(game_id, session.player_id(), String::from(new_game.game_name), uri!(game_view(game_id)));
//             return Ok(Redirect::to(uri!(game_view(game_id))));
//         }
//         _ =>
//         {
//             let err = "Boy howdy, something really went south here.  We received a completely unexpected message type from the GameRunner for creating a game.";
//             return Err(Error::InternalServerError(Template::render("error_pages/500", context! {action_name: "create a new game", error: err})));
//         }
//     }
    

// }

// #[get("/game/<id>")]
// pub async fn game_view(id: Uuid, session: Session, state: &State<Metagame<'_>>) -> Result<Template, Error>
// {
//     let game_name = state.game_name(id);

//     if game_name.is_none()
//     {
//         return Err(Error::NotFound(Template::render("error_pages/404", context!{})));
//     }

//     if state.validate_ownership( session.player_id(), id)
//     {
//         build_gm_view(id, &session, state).await
        
//     }
//     else 
//     {
//         build_player_view(id, &session, state).await
//     } 

// }

// async fn build_player_view(game_id: Uuid, session: &Session, state: &State<Metagame<'_>>) -> Result<Template, Error>
// {
//     let game_name = state.game_name(game_id).unwrap_or(String::from(""));
//     let view: PlayerView;

//     if session.has_character_for(game_id)
//     {
//         match send_and_recv(game_id, Request::GetCharacter(session.character_id(game_id).unwrap()), state.game_runner_pipe.clone()).await?
//         {
//             Outcome::Found(char) => 
//             {
//                 view = PlayerView {player_handle: session.handle_as_ref(), game_id, game_name, character_state: Some(SimpleCharacterView::from(char.unwrap().as_ref()))};
//             }
//             _ => {
//                 let err = "Boy howdy, something really went south here.  We received a completely unexpected message type from the GameRunner for creating a game.";
//                 return Err(Error::InternalServerError(Template::render("error_pages/500", context! {action_name: "create a new game", error: err})));
//             }
//         }
//     }
//     else
//     {
//         view = PlayerView {player_handle: session.handle_as_ref(), game_id, game_name, character_state: None };
//     }

//     // let view = PlayerView {game_id, game_name, character_state: None };

//     Ok(Template::render("player_view", view))
// }

// async fn build_gm_view(game_id: Uuid, _sesion: &Session, state: &State<Metagame<'_>>) -> Result<Template, Error>
// {
//     let outcome = send_and_recv(game_id, Request::GetPcCast, state.game_runner_pipe.clone()).await?;
//     let mut pcs: Vec<SimpleCharacterView>;
//     let mut npcs: Vec<SimpleCharacterView>;
//     let _game_name = state.game_name(game_id).unwrap_or(String::from(""));

//     match outcome
//     {
//         Outcome::CastList(cast) => 
//         {
//             pcs = Vec::with_capacity(cast.len());
//             debug!("Converting Character to SimpleCharacterView for {} records", cast.len());
//             for member in cast
//             {
//                 pcs.push(SimpleCharacterView::from(member.as_ref()));
//             }
//         }
//         _ => 
//         {
//             let err = "Boy howdy, something really went south here.  We received a completely unexpected message type from the GameRunner for creating a game.";
//             return Err(Error::InternalServerError(Template::render("error_pages/500", context! {action_name: "create a new game", error: err})));
//         }
//     }

//     let outcome = send_and_recv(game_id, Request::GetNpcCast, state.game_runner_pipe.clone()).await?;
    
//     match outcome
//     {
//         Outcome::CastList(cast) => 
//         {
//             npcs = Vec::with_capacity(cast.len());
//             for member in cast
//             {
//                 npcs.push(SimpleCharacterView::from(member.as_ref()));
//             }
//         }
//         _ => 
//         {
//             let err = "Boy howdy, something really went south here.  We received a completely unexpected message type from the GameRunner for creating a game.";
//             return Err(Error::InternalServerError(Template::render("error_pages/500", context! {action_name: "create a new game", error: err})));
//         }
//     }

//     return Ok(Template::render("gm_view", GMView { game_id, pcs, npcs }));
// }

// #[post("/game/<id>/add_npc", data="<npc>")]
// pub async fn add_npc(id: Uuid, session: Session, state: &State<Metagame<'_>>, npc: Form<NewCharacter<'_>>) -> Result<Redirect, Error>
// {

//     if !state.validate_ownership(session.player_id(), id)
//     {
//         // TODO: build a 403 tsk tsk tsk kinda
//     }

//     let character = Character::from(npc.into_inner());
    
//     let result = send_and_recv(id, Request::AddCharacter(character), state.game_runner_pipe.clone()).await?;

//     match result
//     {
//         Outcome::CharacterAdded(_) => 
//         {
//             // return Ok(Template::render("added", context!{game_id: id}));
//             return Ok(Redirect::to(uri!(game_view(id))));
//         },
//         Outcome::Error(err) => {return Err(Error::InternalServerError(Template::render("500", context! {action_name: "create a character", error: err.message})))},
//         _ => {return Err(Error::InternalServerError(Template::render("500", context! {action_name: "create a character", error: "The Game replied with an unexpected message."})))}
//     }
// }

// #[post("/game/<id>/add_pc", data="<pc>")]
// pub async fn add_pc(id: Uuid, session: Session, state: &State<Metagame<'_>>, pc: Form<NewCharacter<'_>>) -> Result<Redirect, Error>
// {
//     let character = Character::from(pc.into_inner());

//     let result = send_and_recv(id, Request::AddCharacter(character), state.game_runner_pipe.clone()).await?;
    
//     match result
//     {
//         Outcome::CharacterAdded((_, char_id)) => 
//         {
//             session.add_pc(id, char_id);
//             return Ok(Redirect::to(uri!(game_view(id))));
//         },
//         Outcome::Error(err) => {return Err(Error::InternalServerError(Template::render("500", context! {action_name: "create a character", error: err.message})))},
//         _ => {return Err(Error::InternalServerError(Template::render("500", context! {action_name: "create a character", error: "The Game replied with an unexpected message."})))}
//     }
    
    
// }

// #[get("/<_..>", rank = 11)]
// pub async fn no_session() -> Template
// {
//     Template::render("register", context!{})
// }

// #[derive(FromForm)]
// pub struct UserHandle<'r> {
//     #[field(name = "player_handle")]
//     player_handle: &'r str
// }

// #[post("/gen_session", data = "<submission>")]
// pub async fn new_session(_proof_of_session: NewSessionOutcome, session: Session, submission: Form<UserHandle<'_>>) -> Redirect
// {
//     session.set_handle(String::from(submission.player_handle));
//     Redirect::to(uri!("/"))
// }

// async fn send_and_recv(game_id: Uuid, body: Request, sender: Sender<Message>) -> Result<Outcome, Error>
// {
//     let (their_sender, my_receiver) = channel::<Outcome>();
//     let msg = Message { player_id: None, game_id:Some(game_id), reply_channel: their_sender, msg: body };
//     if let Err(_err) = sender.send(msg).await
//     {
//         return Err(Error::InternalServerError(Template::render("500", context! {action_name: "create a character", error: "The game runner closed its channel."})));
//     }

//     match my_receiver.await 
//     {
//         Ok(outcome) => Ok(outcome),
//         Err(_err) => 
//             Err(Error::InternalServerError(Template::render("500", context! {action_name: "create a character", error: "The reply channel was closed."}))),
//     }
// }