use std::collections::HashMap;

use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;

use crate::tracker::{game::{Game, GameError}, character::Character};



pub async fn game_runner(mut message_queue: Receiver<RequestMessage>)
{

    // set up whatever we're going to use to store running games.  For now, a simple HashMap will do.
    let mut running_games = HashMap::<Uuid, Game>::new();

    while let Some(message) = message_queue.recv().await
    {
        let response: ResponseMessage;
        let channel: tokio::sync::oneshot::Sender<ResponseMessage>;
        match message {
            RequestMessage::New(response_struct) => {
                let game_id = Uuid::new_v4();
                let game = Game::new();
                running_games.insert(game_id, game);
                response = ResponseMessage::Created(game_id);
                channel = response_struct.response;
                // response_struct.response.send(ResponseMessage::Created(game_id)).await;
            },
            RequestMessage::AddCharacter(character_data) => {
                channel = character_data.response;
                match running_games.entry(character_data.game_id)
                {
                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                        let game = entry.get_mut();

                        game.add_cast_member(character_data.character);
                        response = ResponseMessage::CharacterAdded;
                    },
                    std::collections::hash_map::Entry::Vacant(_) => response = game_not_found(character_data.game_id),
                }

            },
            RequestMessage::StartCombat(combat_struct) => {
                match running_games.entry(combat_struct.game_id)
                {
                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                        let game = entry.get_mut();

                        if let Err(result) = game.add_combatants(combat_struct.combatants)
                        {
                            match result.kind
                            {
                                crate::tracker::game::ErrorKind::UnknownCastId => {
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
                            response = ResponseMessage::CombatStarted;
                        }
                        
                    },
                    std::collections::hash_map::Entry::Vacant(_) => {
                        response = game_not_found(combat_struct.game_id);
                    },
                }
                channel = combat_struct.response;
            },
            RequestMessage::AddInitiativeRoll(init_data) => {
                channel = init_data.response;

                match running_games.entry(init_data.game_id)
                {
                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                        let game = entry.get_mut();

                        if let Err(result) = game.accept_initiative_roll(init_data.character_id, init_data.roll)
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
                    std::collections::hash_map::Entry::Vacant(_) => response = game_not_found(init_data.game_id),
                }
            }
        }

        channel.send(response);
    }
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
    AddInitiativeRoll(AddInitiativeRoll),
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
    pub response: tokio::sync::oneshot::Sender<ResponseMessage>,
    pub game_id: Uuid,
    pub combatants: Vec<Uuid>,
}

pub struct AddCharacter
{
    pub response: tokio::sync::oneshot::Sender<ResponseMessage>,
    pub game_id: Uuid,
    pub character: Character,
}

pub struct AddInitiativeRoll
{
    pub response: tokio::sync::oneshot::Sender<ResponseMessage>,
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