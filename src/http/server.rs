
use log::debug;
use rocket::{State, http::{Status, ContentType}, serde::json::Json, post, put, get, };
use tokio::sync::{mpsc::Sender, oneshot::channel};
use tokio::sync::oneshot::Receiver as OneShotReceiver;
use uuid::Uuid;

use crate::{gamerunner::{Event, Outcome, Roll, Message}, http::serde::{NewGame, InitiativeRoll},};

use super::serde::{Character, AddedCharacterJson, NewState, BeginCombat};


#[post("/api/game/new")]
pub async fn new_game(state: &State<Sender<Message>>) -> Result<Json<NewGame>, (Status, String)>
{
    debug!("Request received to generate new game.");
    let msg_channel = state.inner().clone();

    let (runner_sender, response_channel) = channel::<Outcome>();
    // let msg = RequestMessage::New(NewGame{reply_channel: runner_sender});
    let msg = Message { game_id: Uuid::new_v4(), reply_channel: runner_sender, msg: Event::New };

    match do_send(msg, msg_channel, response_channel).await
    {
        Ok(game_msg) => {
            match game_msg {
                Outcome::Created(id) => {
                    debug!("Game created.  ID: {}", id);
                    return Ok(Json(NewGame{game_id:Some(id), game_name: String::from(""), gm_id: None, gm_name: String::from("") }));
                },
                Outcome::Error(err) => {
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
        pc: true,
        metatype: super::serde::Metatypes::Human,
        name: "Mooman",
    };

    return Json(example);
}

#[get("/api/state_demo")]
pub fn get_state_demo() -> Json<NewState>
{
    let mut ids = Vec::<Uuid>::new();

    ids.push(Uuid::new_v4());
    ids.push(Uuid::new_v4());
    ids.push(Uuid::new_v4());

    let change = NewState { to_state: super::serde::State::Combat(BeginCombat { participants: ids }) };

    Json(change)
}

#[post("/api/<id>/character", data = "<character>")]
pub async fn add_new_character(id: Uuid, character: Json<Character<'_>>, state: &State<Sender<Message>>) -> 
    Result<Json<AddedCharacterJson>, (Status, String)>
{
    debug!("Received request to add a character to a game.");

    let (request, response_channel) = channel::<Outcome>();
    let msg_channel = state.inner().clone();
    let game_char = copy_character(&character);

    // TODO: Fix this up proper like.
    // let char_id = game_char.id.clone();

    // let msg = RequestMessage::AddCharacter(AddCharacter{reply_channel: request, game_id: id, character: game_char});
    let msg = Message{ game_id: id, reply_channel: request, msg: Event::AddCharacter(game_char) };

    match do_send(msg, msg_channel, response_channel).await
    {
        Ok(msg) => {
            match msg {
                Outcome::CharacterAdded(char_id) => {
                    let response_json = AddedCharacterJson{ game_id: id.clone(), char_id };
                    return Ok(Json(response_json));        
                },
                Outcome::Error(err) => {
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
pub async fn change_game_state(id: Uuid, new_state: Json<NewState>, state: &State<Sender<Message>>) -> 
    Result<(Status, (ContentType, ())), (Status, String)>
{
    let (game_sender, game_receiver) = channel::<Outcome>();
    let msg_channel = state.inner().clone();
    let msg: Message;

    msg = match &new_state.to_state
    {
        super::serde::State::Combat(combat_data) => {
            // msg = RequestMessage::StartCombat(CombatSetup { reply_channel: game_sender, game_id: id, combatants: combat_data.participants.clone() });
            Message{ game_id: id, reply_channel: game_sender, msg: Event::StartCombat(combat_data.participants.clone()) }
            
        },
        super::serde::State::InitiativeRolls => {
            // RequestMessage::BeginInitiativePhase(SimpleMessage{reply_channel: game_sender, game_id: id})
            Message { game_id: id, reply_channel: game_sender, msg: Event::BeginInitiativePhase }
        },
        super::serde::State::InitiativePass => 
        {
            // RequestMessage::StartCombatRound(SimpleMessage{reply_channel: game_sender, game_id: id})
            Message { game_id: id, reply_channel: game_sender, msg: Event::StartCombatRound }
        },
        super::serde::State::EndOfTurn => {Message { game_id: id, reply_channel: game_sender, msg: Event::BeginEndOfTurn }},
    };

    match do_send(msg, msg_channel, game_receiver).await
    {
        Ok(response_msg) => {
            match response_msg {
                Outcome::Error(err) => {
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
pub async fn add_initiative_roll(id: Uuid, character_init: Json<InitiativeRoll>, state: &State<Sender<Message>>) ->
    Result<(Status, (ContentType, ())), (Status, String)>
{
    let (game_sender, response_channel) = channel::<Outcome>();
    let msg_channel = state.inner().clone();
    // let msg : RequestMessage = RequestMessage::AddInitiativeRoll
    // (
    //     Roll { reply_channel: game_sender, game_id: id, character_id: character_init.char_id, roll: character_init.roll }
    // );
    let msg = Message 
    {
        game_id: id, 
        reply_channel: game_sender, 
        msg: Event::AddInitiativeRoll(Roll{ character_id: character_init.char_id, roll: character_init.roll }) 
    };

    match do_send(msg, msg_channel, response_channel).await
    {
        Ok(response) => {
            match response
            {
                Outcome::Error(err) => {
                    return Err((Status::BadRequest, err.message));
                },

                Outcome::InitiativeRollAdded => {
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

async fn do_send(msg: Message, msg_channel: Sender<Message>, response_channel: OneShotReceiver<Outcome>) 
    -> Result<Outcome, String>
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
    let game_char: crate::tracker::character::Character;
    match character.metatype
    {
        super::serde::Metatypes::Human => game_metatype = crate::tracker::character::Metatypes::Human,
        super::serde::Metatypes::Dwarf => game_metatype = crate::tracker::character::Metatypes::Dwarf,
        super::serde::Metatypes::Elf => game_metatype = crate::tracker::character::Metatypes::Elf,
        super::serde::Metatypes::Troll => game_metatype = crate::tracker::character::Metatypes::Troll,
        super::serde::Metatypes::Orc => game_metatype = crate::tracker::character::Metatypes::Orc,
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