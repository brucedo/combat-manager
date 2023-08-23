
use std::{process::exit, path::PathBuf, fs::{DirEntry, self}, collections::{HashMap, HashSet}, ffi::{OsString, OsStr}, io::ErrorKind, sync::Arc};
use std::io::Error;
use axum::{response::{Response, Redirect}, body::Bytes, extract::{Path, State}, Form};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use axum_macros::debug_handler;
use log::{debug, error};
use uuid::Uuid;
use handlebars::{Handlebars, template};
use tokio::sync::{oneshot::channel, mpsc::Sender};

use crate::{gamerunner::dispatcher::{Message, Request, Outcome}, http::{session::NewSessionOutcome, models::NewGame, modelview::ModelView2}, tracker::character::Character, Configuration};

use super::{models::{GameSummary, GMView, IndexModel, PlayerView, SimpleCharacterView, NewCharacter, Registration}, session::Session, metagame::Metagame, modelview::{StaticView, ModelView}, statics::Statics, server::PlayerId};


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

// #[debug_handler]
pub async fn index(PlayerId(player_id): PlayerId, state: State<Arc<crate::http::state::State<'_>>>) -> Response
{
    debug!("Request for index received.");

    match (
        send_and_recv(Some(player_id), None, Request::Enumerate, state.channel.clone()).await, 
        send_and_recv(Some(player_id), None, Request::PlayerName, state.channel.clone()).await
    )
    {
        
        (Ok(Outcome::Summaries(mut summaries)), Ok(Outcome::PlayerName(player_name))) => {
            debug!("Summary generated and player name retrieved. Player name: {}", player_name);
            let mut model_summaries = Vec::new();
            for (game_uuid, game_name) in summaries.drain(..)
            {
                let mut url = String::from("/game/");
                url.push_str(&game_uuid.to_string());
                debug!("Added game {} to model with url {}.", game_name, url);
                model_summaries.push(GameSummary { game_name, url });
            }

            debug!("Generating response and sending.");

            Response::builder()
                .extension(ModelView2{view: "index", model: Box::from(IndexModel{ player_handle: player_name, summaries: model_summaries })})
                .body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap()
        },
        (Ok(_), Ok(_)) => {
            error!("Either summary could not be generated or the player was not found, either way something went wrong here.");
            Response::builder()
                .extension(ModelView2{view: "500.html", model: Box::from(crate::http::models::Error{ error: "The GameRunner sent unexpected Outcomes." })})
                .body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap()
        },
        (Err(response), _) => response,
        (_, Err(response)) => response
    }
}

pub async fn static_resources(resource: Path<String>) -> Response<axum::body::Empty<Bytes>>
{
    Response::builder()
        .extension(StaticView{view: resource.0})
        .body(axum::body::Empty::<Bytes>::new()).unwrap()
}

pub async fn display_registration_form() -> Response<axum::body::Empty<Bytes>>
{
    debug!("Received request to register new player");
    Response::builder()
        .extension(StaticView{view: String::from("register.html")})
        .body(axum::body::Empty::<Bytes>::new()).unwrap()
}

// #[debug_handler]
pub async fn register_player(state: State<Arc<crate::http::state::State<'_>>>, cookies: CookieJar, form_data: Form<Registration>) -> 
Result<(CookieJar, Redirect), Response<axum::body::Empty<Bytes>>>
{
    // let player_name = form_data.0.player_handle;
    debug!("Processing POST to register request.");

    let (game_sender, game_receiver) = channel();
    let msg = Message { game_id: None, player_id: None, reply_channel: game_sender, msg: Request::NewPlayer(form_data.0.player_handle) };

    if let Err(_) = state.channel.clone().send(msg).await
    {
        debug!("Register player message sent and failed");
        return Err(Response::builder()
            .extension(ModelView2{ view: "500", model: Box::from(crate::http::models::Error{ error: "The GameRunner communication channel has closed unexpectedly.  The administrator will need to restart the system." }) })
            .body(axum::body::Empty::<Bytes>::new()).unwrap());
    }

    debug!("Register player message sent and succeeded");

    match game_receiver.await {
        Ok(Outcome::NewPlayer(player_id)) => {
            debug!("Register player request succeeded");
            Ok((cookies.add(Cookie::new("player_id", player_id.player_id.to_string())), Redirect::to("/")))
        },
        Ok(_) => {
            error!("Register player failed - unexpected return message type.");
            Err(Response::builder()
                .extension(ModelView2{ view: "500", model: Box::from(crate::http::models::Error{ error: "The GameRunner is spewing nonsense.  Someone forgot to pull the defrangulator." })})
                .body(axum::body::Empty::<Bytes>::new()).unwrap())
        }
        Err(_) => {
            error!("Register player failed - channel broke");
            Err(Response::builder()
                .extension(ModelView2{ view: "500", model: Box::from(crate::http::models::Error{error: "The response channel to your registration request errored out.  The administrator will likely need to retart the system."}) })
                .body(axum::body::Empty::<Bytes>::new()).unwrap())
        },
    }
}

pub async fn create_game(PlayerId(player_id): PlayerId, state: State<Arc<crate::http::state::State<'_>>>, new_game: Form<NewGame>) -> Result<Redirect, Response>
{
    debug!("Starting create_game process for game named {}", new_game.game_name);
    let my_sender = state.channel.clone();

    match send_and_recv(Some(player_id), None, Request::NewGame(new_game.0.game_name), my_sender).await
    {
        Ok(outcome) => Ok(Redirect::to("/")),
        Err(response) => Err(response)
    }
}


pub async fn game_view(Path(game_id): Path<Uuid>, PlayerId(player_id): PlayerId, state: State<Arc<crate::http::state::State<'_>>>) -> Response
{
    match send_and_recv(Some(player_id), Some(game_id), Request::HasResourceRole, state.channel.clone()).await
    {
        Ok(Outcome::PlayerRole(crate::gamerunner::authority::Role::RoleGM(_, _))) => {
            match (send_and_recv(Some(player_id), Some(game_id), Request::GetPcCast, state.channel.clone()).await, 
                send_and_recv(Some(player_id), Some(game_id), Request::GetNpcCast, state.channel.clone()).await
            ) {
                (Ok(Outcome::CastList(mut pcs)), Ok(Outcome::CastList(mut npcs))) => {
                    let simple_pcs = pcs.drain(..)
                        .map(|char| {
                            SimpleCharacterView { char_id: char.id.clone(), char_name: char.name.clone(), metatype: char.metatype.clone() } })
                        .collect::<Vec<SimpleCharacterView>>();
                    let simple_npcs = npcs.drain(..)
                        .map(|char| SimpleCharacterView {char_id: char.id.clone(), char_name: char.name.clone(), metatype: char.metatype.clone()})
                        .collect::<Vec<SimpleCharacterView>>();

                    Response::builder().extension(ModelView2{ view: "gm_view", model: Box::from(GMView{ game_id, pcs: simple_pcs, npcs: simple_npcs }) })
                        .body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap()

                },
                (_, _) => {
                    Response::builder()
                        .extension(ModelView2{ view: "500", model: Box::from(crate::http::models::Error{ error: "The GameRunner is spewing nonsense.  Someone forgot to pull the defrangulator." })})
                        .body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap()
                }
            }
        },
        Ok(Outcome::PlayerRole(crate::gamerunner::authority::Role::RolePlayer(_, _))) => {
            match (
                send_and_recv(Some(player_id), Some(game_id), Request::GetPcCast, state.channel.clone()).await,
                send_and_recv(Some(player_id), Some(game_id), Request::PlayerName, state.channel.clone()).await
            )
            {                
                (Ok(Outcome::CastList(mut pcs)), Ok(Outcome::PlayerName(name))) => {
                    let simple_pcs = pcs.drain(..)
                        .map(|char| {
                            SimpleCharacterView { char_id: char.id.clone(), char_name: char.name.clone(), metatype: char.metatype.clone() } })
                        .collect::<Vec<SimpleCharacterView>>();

                    Response::builder().extension(ModelView2{ 
                        view: "player_view", 
                        model: Box::from(PlayerView { player_handle: name, game_id, game_name: todo!(), character_state: Some(simple_pcs) })
                    })
                    .body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap()
                },
                (_, _) => {
                    Response::builder()
                        .extension(ModelView2{ view: "500", model: Box::from(crate::http::models::Error{ error: "The GameRunner is spewing nonsense.  Someone forgot to pull the defrangulator." })})
                        .body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap()                    
                }
            }
        },
        Ok(Outcome::PlayerRole(crate::gamerunner::authority::Role::RoleObserver(_, _))) => todo!(),
        Ok(Outcome::PlayerRole(_)) => todo!(),
        Ok(_) => todo!(),
        Err(response) => response,
    }
    
    // if state.validate_ownership( session.player_id(), id)
    // {
    //     build_gm_view(id, &session, state).await
        
    // }
    // else 
    // {
    //     build_player_view(id, &session, state).await
    // } 

}

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

async fn send_and_recv(player_id: Option<Uuid>, game_id: Option<Uuid>, body: Request, sender: Sender<Message>) -> Result<Outcome, Response>
{
    let (their_sender, my_receiver) = channel::<Outcome>();
    let msg = Message { player_id, game_id,  reply_channel: their_sender, msg: body };
    if let Err(_err) = sender.send(msg).await
    {
        let mut model = HashMap::new();
        model.insert(String::from("error"), String::from("The GameRunner's messaging channel has terminated."));
        return Err(Response::builder()
            .extension(ModelView { view: String::from("500.html"), model: model})
            .body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap());
    }

    match my_receiver.await 
    {
        Ok(outcome) => Ok(outcome),
        Err(_err) => {
            let mut model = HashMap::new();
            model.insert(String::from("error"), String::from("The response channel provided to the runner has terminated."));
            return Err(Response::builder()
                .extension(ModelView { view: String::from("500.html"), model: model})
                .body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap())
        },

    }
}