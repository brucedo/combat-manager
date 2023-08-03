
use std::collections::HashMap;
use std::net::{SocketAddr, IpAddr};
use std::process::exit;
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum::{Router, middleware};
use axum::routing::get;
use axum_extra::extract::CookieJar;
use log::{debug, error};
use serde::Serialize;
use tokio::sync::{mpsc::Sender, oneshot::channel};
use tokio::sync::oneshot::Receiver as OneShotReceiver;
use tower::ServiceBuilder;
use uuid::{Uuid, uuid};

use crate::Configuration;
use crate::http::modelview::{model_view_render, static_file_render};
use crate::http::renders::{initialize_renders, index, static_resources, display_registration_form};
use crate::http::state::State;
use crate::{gamerunner::dispatcher::{Message, Outcome, Roll}, http::{serde::{NewGame, InitiativeRoll}, metagame::Metagame},};

use super::modelview::{StaticView, ModelView};
use super::serde::{Character, AddedCharacterJson, NewState, BeginCombat};


pub async fn start_server(config: &Configuration, game_channel: Sender<Message>)
{
    debug!("start_server() called");

    let (templates, statics) = initialize_renders(config);

    let state = Arc::from(State { handlebars: templates, statics, channel: game_channel });

    let app = Router::new().route("/", get(index))
        .route_layer
        (
            ServiceBuilder::new()
            .layer(middleware::from_fn_with_state(state.clone(), validate_registration::<Body>))
        )
        .route("/static/*resource", get(static_resources))
        .route("/register", get(display_registration_form))
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn_with_state(state.clone(), model_view_render::<Body>))
                .layer(middleware::from_fn_with_state(state.clone(), static_file_render::<Body>))
        )
        .with_state(state);

    let (ip_addr, port) = match (config.bind_addr.parse::<IpAddr>(), config.bind_port.parse::<u16>())
    {
        (Ok(addr), Ok(port)) => (addr, port),
        (_, _) => {error!("Config bind_addr and bind_port settings could not be parsed into a valid IpAddr and port number!"); exit(-1);}
    };
    

    debug!("Attempting to start server on 0.0.0.0:8080");
    axum::Server::bind(&SocketAddr::new(ip_addr, port))
        .serve(app.into_make_service())
        .await
        .unwrap()
}

async fn validate_registration<B>(
    axum::extract::State(state): axum::extract::State<Arc<State<'_>>>, 
    cookie_jar: CookieJar, 
    request: Request<B>, 
    next: Next<B>
) -> Response
{

    let cookie = if let Some(cookie) = cookie_jar.get("player_id"){cookie}
    else
    {
        return Response::builder()
            .status(302)
            .header("Location", "/register")
            .body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap()
    };

    let player_id = if let Ok(player_id) = Uuid::parse_str(cookie.value()) {player_id}
    else
    {
        return Response::builder()
            .status(302)
            .header("Location", "/register")
            .body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap()
    };

    let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel();
    let msg = Message { game_id: None, player_id: Some(player_id), reply_channel: reply_sender, msg: crate::gamerunner::dispatcher::Request::IsRegistered };

    if let Err(_) = state.channel.clone().send(msg).await {
        let mut error_message = HashMap::new();
        error_message.insert(String::from("error"), String::from("The GameRunner messaging channel has broken.  The administrator will likely need to restart the system."));
        return Response::builder().extension(ModelView { view: String::from("500.html"), model: error_message })
            .body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap()
    }

    match reply_receiver.await
    {
        Ok(Outcome::PlayerExists) =>
        {
            return next.run(request).await
        },
        Ok(Outcome::PlayerNotExists) => {
            Response::builder()
            .status(302)
            .header("Location", "/register")
            .body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap()
        },
        Ok(_) => {
            let mut error_message = HashMap::new();
            error_message.insert(String::from("error"), String::from("The GameRunner is returning unreasonable responses for the questions asked.  Likely it has achieved sentience.  I would run."));
            Response::builder().extension(ModelView { view: String::from("500.html"), model: error_message })
                .body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap()
        }
        _ => {
            let mut error_message = HashMap::new();
            error_message.insert(String::from("error"), String::from("The GameRunner messaging channel has broken.  The administrator will likely need to restart the system."));
            Response::builder().extension(ModelView { view: String::from("500.html"), model: error_message })
                .body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap()
        }
        }
}



// #[post("/api/game/new")]
// pub async fn new_game(state: &State<Metagame<'_>>) -> Result<Json<NewGame>, (Status, String)>
// {
//     debug!("Request received to generate new game.");
//     let msg_channel = state.game_runner_pipe.clone();

//     let (runner_sender, response_channel) = channel::<Outcome>();
//     // let msg = RequestMessage::New(NewGame{reply_channel: runner_sender});
//     let msg = Message { player_id: None, game_id: Some(Uuid::new_v4()), reply_channel: runner_sender, msg: Request::New };

//     match do_send(msg, msg_channel, response_channel).await
//     {
//         Ok(game_msg) => {
//             match game_msg {
//                 Outcome::Created(id) => {
//                     debug!("Game created.  ID: {}", id);
//                     return Ok(Json(NewGame{game_id:Some(id), game_name: String::from(""), gm_id: None, gm_name: String::from("") }));
//                 },
//                 Outcome::Error(err) => {
//                     debug!("Game creation error.  Message: {}", err.message);
//                     return Err((Status::InternalServerError, err.message));
//                 },
//                 _ => {unreachable!()}
//             }
//         },
//         Err(err) => {
//             return Err((Status::InternalServerError, err));
//         },
//     }

    
// }

// #[get("/demo")]
// pub fn get_example_char <'r> () -> Json<Character<'r>>
// {
//     let example = Character {
//         pc: true,
//         metatype: super::serde::Metatypes::Human,
//         name: "Mooman",
//     };

//     return Json(example);
// }

// #[get("/state_demo")]
// pub fn get_state_demo() -> Json<NewState>
// {
//     let mut ids = Vec::<Uuid>::new();

//     ids.push(Uuid::new_v4());
//     ids.push(Uuid::new_v4());
//     ids.push(Uuid::new_v4());

//     let change = NewState { to_state: super::serde::State::Combat(BeginCombat { participants: ids }) };

//     Json(change)
// }

// #[post("/<id>/character", data = "<character>")]
// pub async fn add_new_character(id: Uuid, character: Json<Character<'_>>, state: &State<Metagame<'_>>) -> 
//     Result<Json<AddedCharacterJson>, (Status, String)>
// {
//     debug!("Received request to add a character to a game.");

//     let (request, response_channel) = channel::<Outcome>();
//     let msg_channel = state.game_runner_pipe.clone();
//     let game_char = copy_character(&character);

//     // TODO: Fix this up proper like.
//     // let char_id = game_char.id.clone();

//     // let msg = RequestMessage::AddCharacter(AddCharacter{reply_channel: request, game_id: id, character: game_char});
//     let msg = Message{ player_id: None, game_id: Some(id), reply_channel: request, msg: Request::AddCharacter(game_char) };

//     match do_send(msg, msg_channel, response_channel).await
//     {
//         Ok(msg) => {
//             match msg {
//                 Outcome::CharacterAdded((_, char_id)) => {
//                     let response_json = AddedCharacterJson{ game_id: id.clone(), char_id };
//                     return Ok(Json(response_json));        
//                 },
//                 Outcome::Error(err) => {
//                     return Err((Status::BadRequest, err.message));
//                 },
//                 _ => {unreachable!()}
//             }
//         },
//         Err(err) => {
//             debug!("Adding a character failed: {}", err);
//             return Err((Status::BadRequest, err));
//         },
//     }
// }

// #[put("/<id>/state", data = "<new_state>")]
// pub async fn change_game_state(id: Uuid, new_state: Json<NewState>, state: &State<Metagame<'_>>) -> 
//     Result<(Status, (ContentType, ())), (Status, String)>
// {
//     let (game_sender, game_receiver) = channel::<Outcome>();
//     let msg_channel = state.game_runner_pipe.clone();
//     let msg: Message;

//     msg = match &new_state.to_state
//     {
//         super::serde::State::Combat(combat_data) => {
//             // msg = RequestMessage::StartCombat(CombatSetup { reply_channel: game_sender, game_id: id, combatants: combat_data.participants.clone() });
//             Message{ player_id: None, game_id: Some(id), reply_channel: game_sender, msg: Request::StartCombat(combat_data.participants.clone()) }
            
//         },
//         super::serde::State::InitiativeRolls => {
//             // RequestMessage::BeginInitiativePhase(SimpleMessage{reply_channel: game_sender, game_id: id})
//             Message { player_id: None, game_id: Some(id), reply_channel: game_sender, msg: Request::BeginInitiativePhase }
//         },
//         super::serde::State::InitiativePass => 
//         {
//             // RequestMessage::StartCombatRound(SimpleMessage{reply_channel: game_sender, game_id: id})
//             Message { player_id: None, game_id: Some(id), reply_channel: game_sender, msg: Request::StartCombatRound }
//         },
//         super::serde::State::EndOfTurn => {Message { player_id: None, game_id: Some(id), reply_channel: game_sender, msg: Request::BeginEndOfTurn }},
//     };

//     match do_send(msg, msg_channel, game_receiver).await
//     {
//         Ok(response_msg) => {
//             match response_msg {
//                 Outcome::Error(err) => {
//                     return Err((Status::BadRequest, err.message));
//                 }
//                 _ => {
//                     return Ok((Status::Ok, (ContentType::JSON, ())));
//                 }
//         }
//         },
//         Err(err) => {
//             return Err((Status::InternalServerError, err));
//         },
//     }

// }

// #[post("/<id>/initiative", data = "<character_init>")]
// pub async fn add_initiative_roll(id: Uuid, character_init: Json<InitiativeRoll>, state: &State<Metagame<'_>>) ->
//     Result<(Status, (ContentType, ())), (Status, String)>
// {
//     let (game_sender, response_channel) = channel::<Outcome>();
//     let msg_channel = state.game_runner_pipe.clone();
//     // let msg : RequestMessage = RequestMessage::AddInitiativeRoll
//     // (
//     //     Roll { reply_channel: game_sender, game_id: id, character_id: character_init.char_id, roll: character_init.roll }
//     // );
//     let msg = Message 
//     {
//         player_id: None, 
//         game_id: Some(id), 
//         reply_channel: game_sender, 
//         msg: Request::AddInitiativeRoll(Roll{ character_id: character_init.char_id, roll: character_init.roll }) 
//     };

//     match do_send(msg, msg_channel, response_channel).await
//     {
//         Ok(response) => {
//             match response
//             {
//                 Outcome::Error(err) => {
//                     return Err((Status::BadRequest, err.message));
//                 },

//                 Outcome::InitiativeRollAdded => {
//                     return Ok((Status::Ok, (ContentType::JSON, ())));
//                 },
//                 _ => {unreachable!()}
//             }
//         },
//         Err(error_string) => {
//             return Err((Status::InternalServerError, error_string));
//         },
//     }
// }

// async fn do_send(msg: Message, msg_channel: Sender<Message>, response_channel: OneShotReceiver<Outcome>) 
//     -> Result<Outcome, String>
// {

//     match msg_channel.send(msg).await
//     {
//         Ok(_) => {
//             match response_channel.await
//             {
//                 Ok(game_msg) => {return Ok(game_msg)},
//                 Err(_) => {
//                     debug!("One shot send failed.  The one shot may have been closed by the other side with no message.");
//                     return Err(String::from("One shot send failed.  The one shot may have been closed by the other side with no message."))
//                 },
//             }
//         },
//         Err(_) => {
//             debug!("Blocking send failed on game create.  Channel may be defunct.");
//             return Err(String::from("Blocking send failed on game create request.  Channel may have closed."));
//         },
//     }
// }

// fn copy_character(character: &Character) -> crate::tracker::character::Character
// {
//     let game_metatype: crate::tracker::character::Metatypes;
//     let game_char: crate::tracker::character::Character;
//     match character.metatype
//     {
//         super::serde::Metatypes::Human => game_metatype = crate::tracker::character::Metatypes::Human,
//         super::serde::Metatypes::Dwarf => game_metatype = crate::tracker::character::Metatypes::Dwarf,
//         super::serde::Metatypes::Elf => game_metatype = crate::tracker::character::Metatypes::Elf,
//         super::serde::Metatypes::Troll => game_metatype = crate::tracker::character::Metatypes::Troll,
//         super::serde::Metatypes::Orc => game_metatype = crate::tracker::character::Metatypes::Orc,
//     }

//     if character.pc
//     {
//         game_char = crate::tracker::character::Character::new_pc(game_metatype, String::from(character.name));
//     }
//     else
//     {
//         game_char = crate::tracker::character::Character::new_npc(game_metatype, String::from(character.name));
//     }

//     // match character.id {
//     //     Some(id) => {
//     //         game_char.id = id;
//     //     },
//     //     None => {
//     //         game_char.id = Uuid::new_v4();
//     //     },
//     // }
    
    
//    game_char
// }