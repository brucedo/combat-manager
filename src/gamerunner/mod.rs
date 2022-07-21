use std::collections::HashMap;

use log::{debug, error};
use rocket::serde::json::serde_json::map::OccupiedEntry;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot::Sender;
use uuid::Uuid;

use crate::tracker::{game::Game, character::Character};



pub async fn game_runner(mut message_queue: Receiver<RequestMessage>)
{

    // set up whatever we're going to use to store running games.  For now, a simple HashMap will do.
    debug!("Game runner started.");
    let mut running_games = HashMap::<Uuid, Game>::new();

    while let Some(message) = message_queue.recv().await
    {
        debug!("Received request.  processing...");
        let response: ResponseMessage;
        let channel: tokio::sync::oneshot::Sender<ResponseMessage>;
        match message {
            RequestMessage::New(game_details) => {
                debug!("Request is for new game.");
                (channel, response) = new_game(game_details, &mut running_games);
            },
            RequestMessage::Delete(game_details) => {
                debug!("Request is to remove game.");
                (channel, response) = end_game(game_details, &mut running_games);
            }
            RequestMessage::AddCharacter(character_data) => {
                debug!("Request is to add a new character.");
                (channel, response) = add_character(character_data, &mut running_games);
            },
            RequestMessage::StartCombat(details) => {
                debug!("Request is to start the combat phase.");
                (channel, response) = start_combat(details, &mut running_games);
            },
            RequestMessage::AddInitiativeRoll(roll) => {
                debug!("Request is to add an initiative roll.");
                (channel, response) = add_init_roll(roll, &mut running_games);
            },
            RequestMessage::BeginInitiativePhase(game_data) => {
                debug!("Request is to begin the initiative phase.");
                (channel, response) = try_initiative_phase(game_data, &mut running_games)
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
    let channel = game_details.reply_channel;
    let response: ResponseMessage;

    let game_id = Uuid::new_v4();
    let game = Game::new();
    running_games.insert(game_id, game);
    response = ResponseMessage::Created(game_id);


    return (channel, response);
}

fn end_game(game_details: ExistingGame, running_games: &mut HashMap<Uuid, Game>) -> (Sender<ResponseMessage>, ResponseMessage)
{
    let channel = game_details.reply_channel;
    let response: ResponseMessage;

    match running_games.remove(&game_details.game_id)
    {
        Some(_) => {response = ResponseMessage::Destroyed},
        None => {response = ResponseMessage::Error(
            Error{ message: String::from(format!("No game by ID {} exists.", game_details.game_id.clone())), kind: ErrorKind::NoMatchingGame })},
    }

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

            let char_id = game.add_cast_member(character.character);
            response = ResponseMessage::CharacterAdded(char_id);
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

fn try_initiative_phase(game_data: StateChange, running_games: &mut HashMap<Uuid, Game>) -> (Sender<ResponseMessage>, ResponseMessage)
{
    let channel = game_data.reply_channel;
    let response: ResponseMessage;

    match running_games.entry(game_data.game_id)
    {
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            let game = entry.get_mut();
            match game.start_initiative_phase()
            {
                Ok(_) => {
                    response = ResponseMessage::InitiativePhaseStarted;
                },
                Err(game_err) => {
                    let runner_err = Error {kind: ErrorKind::InvalidStateAction, message: game_err.msg};
                    response = ResponseMessage::Error(runner_err);
                },
            }
        },
        std::collections::hash_map::Entry::Vacant(_) => todo!(),
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
    Delete(ExistingGame),
    AddCharacter(AddCharacter),
    StartCombat(CombatSetup),
    AddInitiativeRoll(Roll),
    BeginInitiativePhase(StateChange),
    StartInitiativePass,
    BeginEndOfTurn,
}

pub enum ResponseMessage
{
    Created(Uuid),
    Destroyed,
    Error(Error),
    CharacterAdded(Uuid),
    CombatStarted,
    InitiativePhaseStarted,
    InitiativeRollAdded,
}

pub struct NewGame
{
    pub reply_channel: tokio::sync::oneshot::Sender<ResponseMessage>,
}

pub struct ExistingGame
{
    pub game_id: Uuid,
    pub reply_channel: tokio::sync::oneshot::Sender<ResponseMessage>,
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

pub struct StateChange
{
    pub reply_channel: tokio::sync::oneshot::Sender<ResponseMessage>,
    pub game_id: Uuid,
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

#[cfg(test)]
mod tests
{
    use log::debug;
    use tokio::sync::oneshot::channel;
    use tokio::sync::mpsc::channel as mpsc_channel;
    use tokio::sync::mpsc::Sender;
    use uuid::Uuid;
    

    use crate::gamerunner::{ResponseMessage, game_runner, RequestMessage, NewGame};
    use crate::tracker::character::Character;
    use crate::tracker::character::Metatypes;

    use super::AddCharacter;
    use super::ExistingGame;

    pub fn init() -> Sender<RequestMessage> {
        let _ = env_logger::builder().is_test(true).try_init();
        debug!("Logger should be active.");

        debug!("Created multi-producer, single consumer channel");
        let (sender, receiver) = mpsc_channel(1);

        debug!("About to start game runner.");
        tokio::spawn(async {game_runner(receiver).await;});

        debug!("Runner started, returning.");
        return sender;
    }

    pub async fn add_new_game(game_input_channel: Sender<RequestMessage>) -> Uuid
    {
        let (game_sender, game_receiver) = channel();
        let msg = RequestMessage::New(NewGame{reply_channel: game_sender});

        match game_input_channel.send(msg).await {
            Ok(_) => {
                match game_receiver.await
                {
                    Ok(game_msg) => {
                        match game_msg
                        {
                            ResponseMessage::Created(id) => {return id},
                            _ => {panic!("Received a ResponseMessage enum of an unexpected type.")}
                        }
                    },
                    Err(_) => panic!{"The oneshot channel closed while waiting for reply."},
                }
            },
            Err(_) => panic!("Game input channel closed while waiting for reply."),
        }
    }

    pub fn create_character() -> Character
    {
        let names: [&str; 5] = ["Matrox", "El See-Dee", "BusShock", "Junkyard", "Lo Hax"];
        let metatypes = [Metatypes::Dwarf, Metatypes::Elf, Metatypes::Human, Metatypes::Orc, Metatypes::Troll];
        let mut pc: bool = false;
        if rand::random::<usize>() % 2 == 1 {
            return Character::new_npc(metatypes[rand::random::<usize>() % 5], String::from(names[rand::random::<usize>() % 5]));
        }

        return Character::new_pc(metatypes[rand::random::<usize>() % 5], String::from(names[rand::random::<usize>() % 5]));
        
    }

    #[tokio::test]
    pub async fn test_new_game()
    {
        debug!("Starting new game test.");
        let game_input_channel = init();

        debug!("Creating oneshots");
        // when I send a NewGame message with one half of a oneshot channel...
        let (game_sender, game_receiver) = channel();
        debug!("Creating new game.");
        let msg = RequestMessage::New(NewGame{reply_channel: game_sender});

        debug!("Game created - supposedly.  Await response.");
        // I should get a Uuid on the oneshot reply channel and not an error.
        let send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());
        let response = game_receiver.await;
        debug!("Response received.");
        assert!(response.is_ok());

        // Still find it awkward that I can't just do straight up == on enums without deriving equality traits.  oh well.
        match response.unwrap()
        {
            ResponseMessage::Created(_uuid) => {
                
            },
            _ => {panic!("No other type should have been possible.")}
        }
    }

    #[tokio::test]
    pub async fn test_delete_game()
    {
        let game_input_channel = init();

        let (game_sender, game_receiver) = channel::<ResponseMessage>();

        // when I send a Delete message with one half of a oneshot channel and a game ID that really exists...
        let game_id = add_new_game(game_input_channel.clone()).await;

        let msg = RequestMessage::Delete(ExistingGame{ game_id, reply_channel: game_sender});
        
        let send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());

        let response = game_receiver.await;
        assert!(response.is_ok());

        match response.unwrap()
        {
            ResponseMessage::Destroyed => {/* This is good, nothing to do. */},
            ResponseMessage::Error(err) => {panic!("Received an error: {}", err.message);}
            _ => {panic!("Received ResponseMessage that should not have been generated by request.");}
        }
    }

    #[tokio::test]
    pub async fn delete_invalid_game()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<ResponseMessage>();

        add_new_game(game_input_channel.clone()).await;

        let msg = RequestMessage::Delete(ExistingGame{ game_id: Uuid::new_v4(), reply_channel: game_sender});
        
        let send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());

        let response = game_receiver.await;
        assert!(response.is_ok());

        match response.unwrap()
        {
            ResponseMessage::Destroyed => {panic!("The game deleted somehow - received a Destroyed message instead of an error.");},
            ResponseMessage::Error(_err) => {/* This is good, nothing to do. */}
            _ => {panic!("Received ResponseMessage that should not have been generated by request.");}
        }
    }

    #[tokio::test]
    pub async fn add_character()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<ResponseMessage>();

        let game_id = add_new_game(game_input_channel.clone()).await;

        let character = create_character();

        let msg = RequestMessage::AddCharacter(AddCharacter{ reply_channel: game_sender, game_id, character });
        let send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());

        let response = game_receiver.await;
        assert!(response.is_ok());

        match response.unwrap()
        {
            ResponseMessage::Error(_) => {panic!("This should have been a successful add.")},
            ResponseMessage::CharacterAdded(_) => {},
            _ => {panic!("Another message was triggered, but add new character should result only in error or character added.")}
        }
    }

    
}