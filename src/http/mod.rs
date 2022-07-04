mod api;

use log::debug;
use rocket::{State, http::{Status, ContentType}, serde::json::Json, post, put, get, response::content};
use tokio::sync::{mpsc::Sender, oneshot::channel};
use uuid::Uuid;

use crate::{gamerunner::{RequestMessage, ResponseMessage, NewGame, AddCharacter, CombatSetup, Roll}, http::api::{NewGameJson, InitiativeRoll},};

use self::api::{Character, AddedCharacterJson, StateChange, BeginCombat};


#[post("/api/game/new")]
pub async fn new_game(state: &State<Sender<RequestMessage>>) -> Result<Json<NewGameJson>, (Status, &'static str)>
{
    debug!("Request received to generate new game.");
    let local_sender = state.inner().clone();

    let (runner_sender, runner_receiver) = tokio::sync::oneshot::channel::<ResponseMessage>();
    let msg = RequestMessage::New(NewGame{response: runner_sender});

    match local_sender.send(msg).await
    {
        Ok(_) => 
        {
            match runner_receiver.await
            {
                Ok(game_msg) => {
                    match game_msg
                    {
                        ResponseMessage::Created(id) => {
                            debug!("Game created.  ID: {}", id);
                            return Ok(Json(NewGameJson{game_id: id}));
                        },
                        ResponseMessage::Error(err) => {
                            debug!("Game creation error.  Message: {}", err.message);
                            return Err((Status::InternalServerError, "Game creation encountered some error."));
                        },
                        _ => {unreachable!()}
}
                },
                // The game runner actually does not return an error state here.
                Err(_) => todo!(),
            }
        },
        Err(_) => 
        {
            debug!("Blocking send failed on game create.  Channel may be defunct.");
            return Err((Status::InternalServerError, "Blocking send failed on game create request.  Channel may have closed."));
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
pub async fn add_new_character(id: Uuid, character: Json<Character<'_>>, state: &State<Sender<RequestMessage>>) -> Result<Json<AddedCharacterJson>, (Status, &'static str)>
{
    debug!("Received request to add a character to a game.");

    let (request, response) = channel::<ResponseMessage>();
    let request_sender = state.inner().clone();
    let game_char = copy_character(&character.0);
    let char_id = game_char.id.clone();

    let msg = RequestMessage::AddCharacter(AddCharacter{reply_channel: request, game_id: id, character: game_char});
    
    match request_sender.send(msg).await
    {
        Ok(_) => {
            match response.await
            {
                // GameRunner does not return data for this request - merely "good" or "bad"
                Ok(_msg) => 
                {
                    let response_json = AddedCharacterJson{ game_id: id.clone(), char_id };
                    return Ok(Json(response_json));
                },
                Err(_) => 
                {
                    debug!("Adding a character failed; game not found is likely culprit.");
                    return Err((Status::BadRequest, "Game ID provided not found."));
                },
            }
        },
        Err(_) => 
        {
            debug!("Blocking send failed on game create.  Channel may be defunct.");
            return Err((Status::InternalServerError, "Blocking send failed on game create request.  Channel may have closed."));
        },
    }
}

#[put("/api/<id>/state", data = "<new_state>")]
pub async fn change_game_state(id: Uuid, new_state: Json<StateChange>, state: &State<Sender<RequestMessage>>) -> 
    Result<(Status, (ContentType, &'static str)), (Status, &'static str)>
{
    let (os_sender, os_receiver) = channel::<ResponseMessage>();
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

    match msg_channel.send(msg).await
    {
        Ok(_) => {
            match os_receiver.await {
                Ok(response_msg) => {
                    match response_msg {
                        ResponseMessage::Error(err) => {
                            let msg_str: &str;
                            match err.kind {
                                crate::gamerunner::ErrorKind::NoMatchingGame => {
                                    msg_str = "No matching game found for provided game id.";
                                },
                                crate::gamerunner::ErrorKind::NoSuchCharacter => {
                                    msg_str = "At least one ID in the list does not exist.";
                                },
                                crate::gamerunner::ErrorKind::InvalidStateAction => {
                                    msg_str = "The game is not able to transition to the desired state."
                                },
                            }
                            return Err((Status::BadRequest, msg_str));
                        }
                        _ => {
                            return Ok((Status::Ok, (ContentType::JSON, "")));
                        }
                    }
                    
                },
                Err(_err) => {
                    debug!("Blocking send failed on game create.  Channel may be defunct.");
                    return Err((Status::InternalServerError, "Blocking send failed on game create request.  Channel may have closed."));
                },
            }
        },
        Err(code) => {
            debug!("Blocking send failed on game create.  Channel may be defunct.");
            return Err((Status::InternalServerError, "Blocking send failed on game create request.  Channel may have closed."));
        }
    }
    // let fighting_ids = Vec::from_iter(new_state)

    Ok((Status::Ok, (ContentType::JSON, "")))
}

#[post("/api/<id>/initiative", data = "<character_init>")]
pub async fn add_initiative_roll(id: Uuid, character_init: Json<InitiativeRoll>, state: &State<Sender<RequestMessage>>) ->
    Result<(Status, (ContentType, &'static str)), (Status, &'static str)>
{
    let (os_sender, os_receiver) = channel::<ResponseMessage>();
    let msg_channel = state.inner().clone();
    let msg : RequestMessage = RequestMessage::AddInitiativeRoll
    (
        Roll { reply_channel: os_sender, game_id: id, character_id: character_init.char_id, roll: character_init.roll }
    );

    match msg_channel.send(msg).await
    {
        Ok(_) => {
            
        },
        Err(_) => {
            debug!("Blocking send failed on game create.  Channel may be defunct.");
            return Err((Status::InternalServerError, "Blocking send failed on game create request.  Channel may have closed."));
        },
    }

    Ok((Status::Ok, (ContentType::JSON, "")))
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

    match character.id {
        Some(id) => {
            game_char.id = id;
        },
        None => {
            game_char.id = Uuid::new_v4();
        },
    }
    
    
   game_char
}