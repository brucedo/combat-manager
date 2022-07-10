use std::collections::HashMap;

use log::{debug, error};
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot::Sender;
use uuid::Uuid;

use crate::tracker::{game::Game, character::Character};



pub async fn game_runner(mut message_queue: Receiver<RequestMessage>)
{

    // set up whatever we're going to use to store running games.  For now, a simple HashMap will do.
    let mut running_games = HashMap::<Uuid, Game>::new();

    while let Some(message) = message_queue.recv().await
    {
        let response: ResponseMessage;
        let channel: tokio::sync::oneshot::Sender<ResponseMessage>;
        match message {
            RequestMessage::New(game_details) => {
                (channel, response) = new_game(game_details, &mut running_games);
                // response_struct.response.send(ResponseMessage::Created(game_id)).await;
            },
            RequestMessage::AddCharacter(character_data) => {
                (channel, response) = add_character(character_data, &mut running_games);
            },
            RequestMessage::StartCombat(details) => {
                (channel, response) = start_combat(details, &mut running_games);
            },
            RequestMessage::AddInitiativeRoll(roll) => {
                (channel, response) = add_init_roll(roll, &mut running_games);
            },
            _ => { todo!()}
        }

        if channel.send(response).is_err()
        {
            error!("The return channel has dropped.");
        }
    }
}

fn new_game(game_details: NewGame, running_games: &mut HashMap<Uuid, Game>) -> (Sender<ResponseMessage>, ResponseMessage)
{
    let channel = game_details.response;
    let response: ResponseMessage;

    let game_id = Uuid::new_v4();
    let game = Game::new();
    running_games.insert(game_id, game);
    response = ResponseMessage::Created(game_id);


    return (channel, response);
}

fn add_character(character: AddCharacter, running_games: &mut HashMap<Uuid, Game>) -> (Sender<ResponseMessage>, ResponseMessage)
{
    let channel = character.reply_channel;
    let response: ResponseMessage;

    match running_games.entry(character.game_id)
    {
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            let game = entry.get_mut();

            game.add_cast_member(character.character);
            response = ResponseMessage::CharacterAdded;
        },
        std::collections::hash_map::Entry::Vacant(_) => response = game_not_found(character.game_id),
    }

    return (channel, response);
}

fn start_combat(details: CombatSetup, running_games: &mut HashMap<Uuid, Game>) -> (Sender<ResponseMessage>, ResponseMessage)
{

    let channel = details.reply_channel;
    let response: ResponseMessage;

    match running_games.entry(details.game_id)
    {
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            let game = entry.get_mut();

            if let Err(result) = game.add_combatants(details.combatants)
            {
                match result.kind
                {
                    crate::tracker::game::ErrorKind::UnknownCastId => {
                        response = ResponseMessage::Error
                        (
                            Error 
                            { 
                                message: String::from(format!("Character ID does not exist: {}", result.msg)), 
                                kind: ErrorKind::NoSuchCharacter 
                            }
                        );
                    },
                    _ => {unreachable!()},
                }
            }
            else 
            {
                response = ResponseMessage::CombatStarted;
            }
            
        },
        std::collections::hash_map::Entry::Vacant(_) => {
            response = game_not_found(details.game_id);
        },
    }

    return (channel, response);
}

fn add_init_roll(roll: Roll, running_games: &mut HashMap<Uuid, Game>) -> (Sender<ResponseMessage>, ResponseMessage)
{
    let channel = roll.reply_channel;
    let response: ResponseMessage;

    match running_games.entry(roll.game_id)
    {
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            let game = entry.get_mut();

            if let Err(result) = game.accept_initiative_roll(roll.character_id, roll.roll)
            {
                match result.kind
                {
                    crate::tracker::game::ErrorKind::InvalidStateAction => {
                        response = ResponseMessage::Error
                        (
                            Error 
                            { 
                                message: String::from(format!("The game is not in the correct state to take initiative rolls.")), 
                                kind: ErrorKind::InvalidStateAction 
                            }
                        );
                    },
                    crate::tracker::game::ErrorKind::NoCombatants => {
                        response = ResponseMessage::Error
                        (
                            Error 
                            { 
                                message: String::from(format!("Character ID does not exist: {}", result.msg)), 
                                kind: ErrorKind::NoMatchingGame 
                            }
                        );
                    },
                    _ => {unreachable!()},
                }
            }
            else
            {
                response = ResponseMessage::InitiativeRollAdded;
            }
        },
        std::collections::hash_map::Entry::Vacant(_) => response = game_not_found(roll.game_id),
    }

    return (channel, response);



}

fn game_not_found(id: Uuid) -> ResponseMessage
{
    ResponseMessage::Error
    (
        Error 
        { 
            message: String::from(format!("The ID provided ({}) has no associated game.", id)), 
            kind: ErrorKind::NoMatchingGame 
        }
    )
}

pub enum RequestMessage
{
    New(NewGame),
    AddCharacter(AddCharacter),
    StartCombat(CombatSetup),
    AddInitiativeRoll(Roll),
    AcceptInitiativeRolls,
    StartInitiativePass,
    BeginEndOfTurn,
}

pub enum ResponseMessage
{
    Created(Uuid),
    Error(Error),
    CharacterAdded,
    CombatStarted,
    InitiativeRollAdded,
}

pub struct NewGame
{
    pub response: tokio::sync::oneshot::Sender<ResponseMessage>,
}

pub struct CombatSetup
{
    pub reply_channel: tokio::sync::oneshot::Sender<ResponseMessage>,
    pub game_id: Uuid,
    pub combatants: Vec<Uuid>,
}

pub struct AddCharacter
{
    pub reply_channel: tokio::sync::oneshot::Sender<ResponseMessage>,
    pub game_id: Uuid,
    pub character: Character,
}

pub struct Roll
{
    pub reply_channel: tokio::sync::oneshot::Sender<ResponseMessage>,
    pub game_id: Uuid,
    pub character_id: Uuid,
    pub roll: i8,
}

pub struct Error
{
    pub message: String,
    pub kind: ErrorKind,
}

pub enum ErrorKind
{
    NoMatchingGame,
    NoSuchCharacter,
    InvalidStateAction,
}