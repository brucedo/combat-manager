mod api;

use log::debug;
use rocket::{State, http::{Status, ContentType}, serde::json::Json, post, put, get, };
use tokio::sync::{mpsc::Sender, oneshot::channel};
use tokio::sync::oneshot::Receiver as OneShotReceiver;
use uuid::Uuid;

use crate::{gamerunner::{RequestMessage, ResponseMessage, NewGame, AddCharacter, CombatSetup, Roll}, http::api::{NewGameJson, InitiativeRoll},};

use self::api::{Character, AddedCharacterJson, StateChange, BeginCombat};


#[post("/api/game/new")]
pub async fn new_game(state: &State<Sender<RequestMessage>>) -> Result<Json<NewGameJson>, (Status, String)>
{
    debug!("Request received to generate new game.");
    let msg_channel = state.inner().clone();

    let (runner_sender, response_channel) = channel::<ResponseMessage>();
    let msg = RequestMessage::New(NewGame{response: runner_sender});

    match do_send(msg, msg_channel, response_channel).await
    {
        Ok(game_msg) => {
            match game_msg {
                ResponseMessage::Created(id) => {
                    debug!("Game created.  ID: {}", id);
                    return Ok(Json(NewGameJson{game_id: id}));
                },
                ResponseMessage::Error(err) => {
                    debug!("Game creation error.  Message: {}", err.message);
                    return Err((Status::InternalServerError, err.message));
                },
                _ => {unreachable!()}
            }
        },
        Err(err) => {
            return Err((Status::InternalServerError, err));
        },
    }

    
}

#[get("/api/demo")]
pub fn get_example_char <'r> () -> Json<Character<'r>>
{
    let example = Character {
        id: Some(Uuid::new_v4()),
        pc: true,
        metatype: api::Metatypes::Human,
        name: "Mooman",
    };

    return Json(example);
}

#[get("/api/state_demo")]
pub fn get_state_demo() -> Json<StateChange>
{
    let mut ids = Vec::<Uuid>::new();

    ids.push(Uuid::new_v4());
    ids.push(Uuid::new_v4());
    ids.push(Uuid::new_v4());

    let change = StateChange { to_state: api::State::Combat(BeginCombat { participants: ids }) };

    Json(change)
}

#[post("/api/<id>/character", data = "<character>")]
pub async fn add_new_character(id: Uuid, character: Json<Character<'_>>, state: &State<Sender<RequestMessage>>) -> 
    Result<Json<AddedCharacterJson>, (Status, String)>
{
    debug!("Received request to add a character to a game.");

    let (request, response_channel) = channel::<ResponseMessage>();
    let msg_channel = state.inner().clone();
    let game_char = copy_character(&character.0);

    // TODO: Fix this up proper like.
    // let char_id = game_char.id.clone();

    let msg = RequestMessage::AddCharacter(AddCharacter{reply_channel: request, game_id: id, character: game_char});

    match do_send(msg, msg_channel, response_channel).await
    {
        Ok(msg) => {
            match msg {
                ResponseMessage::CharacterAdded(char_id) => {
                    let response_json = AddedCharacterJson{ game_id: id.clone(), char_id };
                    return Ok(Json(response_json));        
                },
                ResponseMessage::Error(err) => {
                    return Err((Status::BadRequest, err.message));
                },
                _ => {unreachable!()}
            }
        },
        Err(err) => {
            debug!("Adding a character failed: {}", err);
            return Err((Status::BadRequest, err));
        },
    }
}

#[put("/api/<id>/state", data = "<new_state>")]
pub async fn change_game_state(id: Uuid, new_state: Json<StateChange>, state: &State<Sender<RequestMessage>>) -> 
    Result<(Status, (ContentType, ())), (Status, String)>
{
    let (os_sender, response_channel) = channel::<ResponseMessage>();
    let msg_channel = state.inner().clone();
    let msg: RequestMessage;

    match &new_state.to_state
    {
        api::State::Combat(combat_data) => {
            msg = RequestMessage::StartCombat(CombatSetup { reply_channel: os_sender, game_id: id, combatants: combat_data.participants.clone() });
        },
        api::State::InitiativeRolls => {msg = RequestMessage::AcceptInitiativeRolls},
        api::State::InitiativePass => {msg = RequestMessage::StartInitiativePass},
        api::State::EndOfTurn => {msg = RequestMessage::BeginEndOfTurn},
    }

    match do_send(msg, msg_channel, response_channel).await
    {
        Ok(response_msg) => {
            match response_msg {
                ResponseMessage::Error(err) => {
                    return Err((Status::BadRequest, err.message));
                }
                _ => {
                    return Ok((Status::Ok, (ContentType::JSON, ())));
                }
        }
        },
        Err(err) => {
            return Err((Status::InternalServerError, err));
        },
    }

}

#[post("/api/<id>/initiative", data = "<character_init>")]
pub async fn add_initiative_roll(id: Uuid, character_init: Json<InitiativeRoll>, state: &State<Sender<RequestMessage>>) ->
    Result<(Status, (ContentType, ())), (Status, String)>
{
    let (game_sender, response_channel) = channel::<ResponseMessage>();
    let msg_channel = state.inner().clone();
    let msg : RequestMessage = RequestMessage::AddInitiativeRoll
    (
        Roll { reply_channel: game_sender, game_id: id, character_id: character_init.char_id, roll: character_init.roll }
    );

    match do_send(msg, msg_channel, response_channel).await
    {
        Ok(response) => {
            match response
            {
                ResponseMessage::Error(err) => {
                    return Err((Status::BadRequest, err.message));
                },

                ResponseMessage::InitiativeRollAdded => {
                    return Ok((Status::Ok, (ContentType::JSON, ())));
                },
                _ => {unreachable!()}
            }
        },
        Err(error_string) => {
            return Err((Status::InternalServerError, error_string));
        },
    }
}

async fn do_send(msg: RequestMessage, msg_channel: Sender<RequestMessage>, response_channel: OneShotReceiver<ResponseMessage>) 
    -> Result<ResponseMessage, String>
{

    match msg_channel.send(msg).await
    {
        Ok(_) => {
            match response_channel.await
            {
                Ok(game_msg) => {return Ok(game_msg)},
                Err(_) => {
                    debug!("One shot send failed.  The one shot may have been closed by the other side with no message.");
                    return Err(String::from("One shot send failed.  The one shot may have been closed by the other side with no message."))
                },
            }
        },
        Err(_) => {
            debug!("Blocking send failed on game create.  Channel may be defunct.");
            return Err(String::from("Blocking send failed on game create request.  Channel may have closed."));
        },
    }
}

fn copy_character(character: &Character) -> crate::tracker::character::Character
{
    let game_metatype: crate::tracker::character::Metatypes;
    let mut game_char: crate::tracker::character::Character;
    match character.metatype
    {
        api::Metatypes::Human => game_metatype = crate::tracker::character::Metatypes::Human,
        api::Metatypes::Dwarf => game_metatype = crate::tracker::character::Metatypes::Dwarf,
        api::Metatypes::Elf => game_metatype = crate::tracker::character::Metatypes::Elf,
        api::Metatypes::Troll => game_metatype = crate::tracker::character::Metatypes::Troll,
        api::Metatypes::Orc => game_metatype = crate::tracker::character::Metatypes::Orc,
    }

    if character.pc
    {
        game_char = crate::tracker::character::Character::new_pc(game_metatype, String::from(character.name));
    }
    else
    {
        game_char = crate::tracker::character::Character::new_npc(game_metatype, String::from(character.name));
    }

    // match character.id {
    //     Some(id) => {
    //         game_char.id = id;
    //     },
    //     None => {
    //         game_char.id = Uuid::new_v4();
    //     },
    // }
    
    
   game_char
}