use std::{collections::HashMap, process::Output};

use log::{debug, error};
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
                channel = character_data.reply_channel;
                let character = character_data.character;
                response = find_game_and_act(&mut running_games, character_data.game_id, | game | {add_character(character, game)});
                // (channel, response) = add_character(character_data, &mut running_games);
            },
            RequestMessage::StartCombat(details) => {
                debug!("Request is to start the combat phase.");
                channel = details.reply_channel;
                let combatants = details.combatants;
                response = find_game_and_act(&mut running_games, details.game_id, | game | {start_combat(combatants, game)})
                // (channel, response) = start_combat(details, &mut running_games);
            },
            RequestMessage::AddInitiativeRoll(roll) => {
                debug!("Request is to add an initiative roll.");
                channel = roll.reply_channel;
                let character_id = roll.character_id;
                let initiative_value = roll.roll;
                // (channel, response) = add_init_roll(roll, &mut running_games);
                response = find_game_and_act(&mut running_games, roll.game_id, | game | { add_init_roll(character_id, initiative_value, game)});
            },
            RequestMessage::BeginInitiativePhase(game_data) => {
                debug!("Request is to begin the initiative phase.");
                (channel, response) = try_initiative_phase(game_data, &mut running_games)
            },
            RequestMessage::StartCombatRound(msg) => {
                debug!("Request is to begin a combat round.");
                (channel, response) = try_begin_combat(msg, &mut running_games)
            },
            RequestMessage::AdvanceTurn(msg) => {
                debug!("Request is to advance to the next event in the pass.");
                let id = msg.game_id;
                channel = msg.reply_channel;
                // let response = find_game_and_act(&mut running_games, id, | game | {return try_advance_turn(msg, game)});
                response = find_game_and_act(&mut running_games, id, try_advance_turn);
            }
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

fn add_character(character: Character, game: &mut Game) -> ResponseMessage
{
    let response: ResponseMessage;

    let char_id = game.add_cast_member(character);
    return ResponseMessage::CharacterAdded(char_id);

}

// fn start_combat(details: CombatSetup, running_games: &mut HashMap<Uuid, Game>) -> (Sender<ResponseMessage>, ResponseMessage)
fn start_combat(combatants: Vec<Uuid>, game: &mut Game) -> ResponseMessage
{

    let response: ResponseMessage;

    if let Err(result) = game.add_combatants(combatants)
    {
        match result.kind
        {
            crate::tracker::game::ErrorKind::UnknownCastId => {
                response = ResponseMessage::Error
                (
                    Error 
                    { 
                        message: result.msg, 
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

    return response;

}

fn try_initiative_phase(game_data: SimpleMessage, running_games: &mut HashMap<Uuid, Game>) -> (Sender<ResponseMessage>, ResponseMessage)
{
    let channel = game_data.reply_channel;
    let response: ResponseMessage;

    match running_games.entry(game_data.game_id)
    {
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            let game = entry.get_mut();
            debug!("Game exists.");
            match game.start_initiative_phase()
            {
                Ok(_) => {
                    debug!("Non-error returned from game.start_initiative_phase()");
                    response = ResponseMessage::InitiativePhaseStarted;
                },
                Err(game_err) => {
                    debug!("Error returned from game.start_initiative_phase()");
                    let runner_err = Error {kind: ErrorKind::InvalidStateAction, message: game_err.msg};
                    response = ResponseMessage::Error(runner_err);
                },
            }
        },
        std::collections::hash_map::Entry::Vacant(_) => todo!(),
    }

    return (channel, response);
}

fn add_init_roll(character_id: Uuid, roll: i8, game: &mut Game) -> ResponseMessage
{
    // let channel = roll.reply_channel;
    let response: ResponseMessage;

    // match running_games.entry(roll.game_id)
    // {
    //     std::collections::hash_map::Entry::Occupied(mut entry) => {
    //         let game = entry.get_mut();

    if let Err(result) = game.accept_initiative_roll(character_id, roll)
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
    //     },
    //     std::collections::hash_map::Entry::Vacant(_) => response = game_not_found(roll.game_id),
    // }

    // return (channel, response);
    return response;

}

fn try_begin_combat(msg: SimpleMessage, running_games: &mut HashMap<Uuid, Game>) -> (Sender<ResponseMessage>, ResponseMessage)
{
    let game_id = msg.game_id;
    let reply_channel = msg.reply_channel;
    let response: ResponseMessage;

    match running_games.entry(game_id)
    {
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            let game = entry.get_mut();

            if let Err(err) = game.start_combat_rounds()
            {
                match err.kind
                {
                    crate::tracker::game::ErrorKind::InvalidStateAction => {
                        response = ResponseMessage::Error(Error{ message: err.msg, kind: ErrorKind::InvalidStateAction })
                    },
                    _ => {unreachable!()}
                }
            }
            else 
            {
                response = ResponseMessage::CombatRoundStarted;    
            }
        },
        std::collections::hash_map::Entry::Vacant(_) =>
        {
            response = game_not_found(game_id);
        }
    }

    return (reply_channel, response);
}

pub fn try_advance_turn(game: &mut Game) -> ResponseMessage
{
    let response: ResponseMessage;

    if let Err(err) = game.next_initiative()
    {
        match err.kind
        {
            crate::tracker::game::ErrorKind::InvalidStateAction => 
            {
                response = ResponseMessage::Error(Error{message: err.msg, kind: ErrorKind::InvalidStateAction});
            },
            crate::tracker::game::ErrorKind::UnresolvedCombatant => 
            {
                response = ResponseMessage::Error(Error{message: err.msg, kind: ErrorKind::CannotAdvanceTurn})
            },
            _ => {unreachable!("Should not receive any other error from stepping the initiative forward.")}
        }
    }
    else
    {
        let mut up = match game.waiting_for(){ Some(filled) => filled, None => Vec::<Uuid>::new() };
        let mut on_deck = match game.on_deck(){ Some(filled) => filled, None => Vec::<Uuid>::new() };

        response = ResponseMessage::TurnAdvanced(TurnAdvanced{ up: up, on_deck: on_deck });
    }

    return response;
}

fn find_game_and_act<F>(running_games: &mut HashMap<Uuid, Game>, game_id: Uuid, action: F) -> ResponseMessage
where
    F: FnOnce(&mut Game) -> ResponseMessage
{
    let response: ResponseMessage;

    match running_games.entry(game_id)
    {
        std::collections::hash_map::Entry::Occupied(mut game) => 
        {
            response = action(game.get_mut());
        },
        std::collections::hash_map::Entry::Vacant(_) => {response = game_not_found(game_id)},
    }

    return response;
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
    BeginInitiativePhase(SimpleMessage),
    QueryInitiativePhase(SimpleMessage),
    StartCombatRound(SimpleMessage),
    AdvanceTurn(SimpleMessage),
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
    InitiativeStatus(InitiativeState),
    CombatRoundStarted,
    TurnAdvanced(TurnAdvanced)
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

pub struct InitiativeState
{
    pub waiting: bool,
    pub remaining: Vec<Uuid>
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

pub struct SimpleMessage
{
    pub reply_channel: tokio::sync::oneshot::Sender<ResponseMessage>,
    pub game_id: Uuid,
}

pub struct Error
{
    pub message: String,
    pub kind: ErrorKind,
}

pub struct TurnAdvanced
{
    pub up: Vec<Uuid>,
    pub on_deck: Vec<Uuid>,
}

#[derive(PartialEq)]
pub enum ErrorKind
{
    NoMatchingGame,
    NoSuchCharacter,
    InvalidStateAction,
    CannotAdvanceTurn,
}

#[cfg(test)]
mod tests
{
    use core::panic;

    use crate::gamerunner::CombatSetup;
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
    use super::ErrorKind;
    use super::Roll;
    use super::SimpleMessage;

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

    pub async fn add_new_game(game_input_channel: &Sender<RequestMessage>) -> Uuid
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
        let pc: bool = false;
        if rand::random::<usize>() % 2 == 1 {
            return Character::new_npc(metatypes[rand::random::<usize>() % 5], String::from(names[rand::random::<usize>() % 5]));
        }

        return Character::new_pc(metatypes[rand::random::<usize>() % 5], String::from(names[rand::random::<usize>() % 5]));
        
    }

    async fn create_and_add_char(game_input_channel: &Sender<RequestMessage>, game_id: Uuid) -> Uuid
    {
        let (game_sender, game_receiver) = channel::<ResponseMessage>();

        let character = create_character();

        let msg = RequestMessage::AddCharacter(AddCharacter{ reply_channel: game_sender, game_id, character });
        let send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());

        let response = game_receiver.await;
        assert!(response.is_ok());

        match response
        {
            Ok(msg) => {
                match msg
                {
                    ResponseMessage::CharacterAdded(id) => {return id;}
                    _ => {panic!("Attempt to add character for test failed.");}
                }
            },
            Err(_) => {panic!("Channel closed.")}
        }
    }

    #[tokio::test]
    pub async fn creating_the_first_new_game_will_generate_created_message()
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
    pub async fn deleting_a_game_with_its_id_will_generate_destroyed_message()
    {
        let game_input_channel = init();

        let (game_sender, game_receiver) = channel::<ResponseMessage>();

        // when I send a Delete message with one half of a oneshot channel and a game ID that really exists...
        let game_id = add_new_game(&game_input_channel).await;

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
    pub async fn deleting_a_game_with_an_unknown_id_will_generate_no_matching_game()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<ResponseMessage>();

        add_new_game(&game_input_channel).await;

        let msg = RequestMessage::Delete(ExistingGame{ game_id: Uuid::new_v4(), reply_channel: game_sender});
        
        let send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());

        let response = game_receiver.await;
        assert!(response.is_ok());

        match response.unwrap()
        {
            ResponseMessage::Destroyed => {panic!("The game deleted somehow - received a Destroyed message instead of an error.");},
            ResponseMessage::Error(err) => 
            {
                assert!(err.kind == ErrorKind::NoMatchingGame);
            }
            _ => {panic!("Received ResponseMessage that should not have been generated by request.");}
        }
    }

    #[tokio::test]
    pub async fn adding_a_new_character_to_a_valid_game_roster_generates_character_added_message()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<ResponseMessage>();

        let game_id = add_new_game(&game_input_channel).await;

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

    #[tokio::test]
    pub async fn adding_a_character_to_a_non_extant_game_will_generate_no_matching_game()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<ResponseMessage>();

        let _ = add_new_game(&game_input_channel).await;

        let character = create_character();

        let msg = RequestMessage::AddCharacter(AddCharacter{ reply_channel: game_sender, game_id: Uuid::new_v4(), character});
        let send_state = game_input_channel.send(msg).await;

        assert!(send_state.is_ok());

        let response = game_receiver.await;
        assert!(response.is_ok());

        match response.unwrap()
        {
            ResponseMessage::Error(err) => 
            { 
                assert!(err.kind == ErrorKind::NoMatchingGame);
            },
            ResponseMessage::CharacterAdded(_) => {panic!("This add should have failed - should have received Error rather than CharacterAdded.")},
            _ => {panic!("Another message was triggered, but add new character should result only in error or character added.")}
        }
    }

    #[tokio::test]
    pub async fn starting_combat_with_registered_characters_will_generate_combat_started_message()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<ResponseMessage>();

        let game_id = add_new_game(&game_input_channel).await;

        let character1 = create_and_add_char(&game_input_channel, game_id).await;
        let character2 = create_and_add_char(&game_input_channel, game_id).await;
        let character3 = create_and_add_char(&game_input_channel, game_id).await;
        let character4 = create_and_add_char(&game_input_channel, game_id).await;

        let combatants = vec![character1, character2, character3, character4];

        let msg = RequestMessage::StartCombat(CombatSetup{reply_channel: game_sender, game_id, combatants});

        let response = game_input_channel.send(msg).await;

        assert!(response.is_ok());
        
        match game_receiver.await
        {
            Ok(msg) => {
                match msg {
                    ResponseMessage::CombatStarted => {} // Success, nothing in the response to test.
                    _ => {panic!("Combat failed to start; a different message was returned by the Game.")}
                }
            },
            Err(_) => {
                panic!("A channel error occurred during the test.")
            }
        }
    }

    #[tokio::test]
    pub async fn starting_combat_with_unregistered_characters_will_generate_no_such_character_error()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<ResponseMessage>();

        let game_id = add_new_game(&game_input_channel).await;

        let _character1 = create_and_add_char(&game_input_channel, game_id).await;
        let _character2 = create_and_add_char(&game_input_channel, game_id).await;
        let _character3 = create_and_add_char(&game_input_channel, game_id).await;
        let _character4 = create_and_add_char(&game_input_channel, game_id).await;

        let combatants = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];

        let msg = RequestMessage::StartCombat(CombatSetup{reply_channel: game_sender, game_id, combatants});

        let response = game_input_channel.send(msg).await;

        assert!(response.is_ok());
        
        match game_receiver.await
        {
            Ok(msg) => {
                match msg {
                    ResponseMessage::CombatStarted => {panic!("The combat stage was started, but the Ids provided should not be characters.");}
                    ResponseMessage::Error(err) => {
                        match err.kind
                        {
                            ErrorKind::NoSuchCharacter => {

                            }
                            _ => {panic!("Unexpected error message returned.");}
                        }
                    }
                    _ => {panic!("Combat failed to start; a different message was returned by the Game.")}
                }
            },
            Err(_) => {
                panic!("A channel error occurred during the test.")
            }
        }

    }

    #[tokio::test]
    pub async fn starting_combat_with_no_combatants_will_generate_combat_started_message()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<ResponseMessage>();

        let game_id = add_new_game(&game_input_channel).await;

        let _character1 = create_and_add_char(&game_input_channel, game_id).await;
        let _character2 = create_and_add_char(&game_input_channel, game_id).await;
        let _character3 = create_and_add_char(&game_input_channel, game_id).await;
        let _character4 = create_and_add_char(&game_input_channel, game_id).await;

        let msg = RequestMessage::StartCombat(CombatSetup{reply_channel: game_sender, game_id, combatants: Vec::<Uuid>::new()});

        let response = game_input_channel.send(msg).await;

        assert!(response.is_ok()); // It is entirely acceptable to start a combat with no combatants.  Individual combatants can be added later,
        // or another batch of combatants can be added later.
    }

    #[tokio::test]
    pub async fn starting_combat_with_an_unregistered_game_id_will_generate_no_matching_game()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<ResponseMessage>();

        let game_id = add_new_game(&game_input_channel).await;

        let character1 = create_and_add_char(&game_input_channel, game_id).await;
        let character2 = create_and_add_char(&game_input_channel, game_id).await;
        let character3 = create_and_add_char(&game_input_channel, game_id).await;
        let character4 = create_and_add_char(&game_input_channel, game_id).await;

        let combatants = vec![character1, character2, character3, character4];

        let msg = RequestMessage::StartCombat(CombatSetup{reply_channel: game_sender, game_id: Uuid::new_v4(), combatants});

        let response = game_input_channel.send(msg).await;

        assert!(response.is_ok());
        
        match game_receiver.await
        {
            Ok(msg) => {
                match msg {
                    ResponseMessage::CombatStarted => {panic!("This should have returned an error!");}
                    ResponseMessage::Error(err) => {
                        match err.kind
                        {
                            ErrorKind::NoMatchingGame => {}
                            _ => {panic!("Wrong kind: should have caught the incorrect game UUID.")}
                        }
                    } 
                    _ => {panic!("Combat failed to start; a different message was returned by the Game.")}
                }
            },
            Err(_) => {
                panic!("A channel error occurred during the test.")
            }
        }

    }

    #[tokio::test]
    pub async fn sending_begin_initiative_phase_to_combat_readied_game_generates_initiative_phase_started()
    {
        let game_input_channel = init();
        let (game_sender, _game_receiver) = channel::<ResponseMessage>();

        let game_id = add_new_game(&game_input_channel).await;

        let character1 = create_and_add_char(&game_input_channel, game_id).await;
        let character2 = create_and_add_char(&game_input_channel, game_id).await;
        let character3 = create_and_add_char(&game_input_channel, game_id).await;
        let character4 = create_and_add_char(&game_input_channel, game_id).await;
        let combatants = vec![character1, character2, character3, character4];

        let msg = RequestMessage::StartCombat(CombatSetup{reply_channel: game_sender, game_id, combatants});

        let _response = game_input_channel.send(msg).await;

        let (game_sender, game_receiver) = channel::<ResponseMessage>();

        let msg = RequestMessage::BeginInitiativePhase(SimpleMessage{game_id, reply_channel:game_sender});

        let response = game_input_channel.send(msg).await;

        assert!(response.is_ok());

        match game_receiver.await
        {
            Ok(msg) => {
                match msg
                {
                    ResponseMessage::InitiativePhaseStarted => {} // all is good
                    _ => {panic!("Received an unexpected ResponseMessage");}
                }
            }, 
            Err(_) => {
                panic!("Receiver channel errored.")
            }        
        }
        
    }

    #[tokio::test]
    pub async fn sending_begin_initiative_phase_to_game_with_combatantless_active_combat_generates_invalid_state_action()
    {
        let game_input_channel = init();
        let (game_sender, _game_receiver ) = channel::<ResponseMessage>();

        let game_id = add_new_game(&game_input_channel).await;

        let _character1 = create_and_add_char(&game_input_channel, game_id).await;
        let _character2 = create_and_add_char(&game_input_channel, game_id).await;
        let _character3 = create_and_add_char(&game_input_channel, game_id).await;

        let msg = RequestMessage::StartCombat(CombatSetup{reply_channel: game_sender, game_id, combatants: Vec::<Uuid>::new()});

        let _response = game_input_channel.send(msg).await;

        let (game_sender, game_receiver) = channel::<ResponseMessage>();

        let msg = RequestMessage::BeginInitiativePhase(SimpleMessage{game_id, reply_channel:game_sender});

        let response = game_input_channel.send(msg).await;

        assert!(response.is_ok());

        match game_receiver.await
        {
            Ok(msg) => {
                match msg
                {
                    ResponseMessage::Error(kind) => {
                        if kind.kind != ErrorKind::InvalidStateAction
                        {
                            panic!("Expected InvalidStateAction error type to signify no characters in the combat set.");
                        }
                    } // This is correct
                    _ => {panic!("Expected an error when starting initiative round with no combatants - received non-error result!")}
                }
            },
            Err(_) => {
                panic!("Receiver channel errored.")
            }
        }
    }

    #[tokio::test]
    pub async fn sending_add_initiative_roll_with_valid_game_id_and_registered_combatant_id_generates_initiative_roll_added()
    {
        let game_input_channel = init();
        let (game_sender, _game_receiver) = channel::<ResponseMessage>();

        let game_id = add_new_game(&game_input_channel).await;

        let character1 = create_and_add_char(&game_input_channel, game_id).await;
        let character2 = create_and_add_char(&game_input_channel, game_id).await;

        let msg = RequestMessage::StartCombat(CombatSetup{reply_channel: game_sender, game_id, combatants: vec![character1, character2]});

        assert!(game_input_channel.send(msg).await.is_ok());

        let (game_sender, _game_receiver) = channel::<ResponseMessage>();
        let msg = RequestMessage::BeginInitiativePhase(SimpleMessage{game_id, reply_channel: game_sender});
        assert!(game_input_channel.send(msg).await.is_ok());

        let (game_sender, game_receiver) = channel::<ResponseMessage>();
        let roll: Roll = Roll{ reply_channel: game_sender, game_id, character_id: character1, roll: 13 };
        let msg = RequestMessage::AddInitiativeRoll(roll);
        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await
        {
            Ok(response) => {
                match response
                {
                    ResponseMessage::InitiativeRollAdded => {},
                    _ => {
                        panic!("Unexpected ResponseMessage - should have been InitiativeRollAdded.")
                    }
                }
            },
            Err(_) => {
                panic!("The oneshot channel errored out before the GameRunner could send a response.");
            } 
        }

    }

    #[tokio::test]
    pub async fn sending_add_initiative_roll_for_all_registered_combatants_generates_initiative_roll_added_for_each()
    {
        let game_input_channel = init();
        let (game_sender, _game_receiver) = channel::<ResponseMessage>();

        let game_id = add_new_game(&game_input_channel).await;

        let character1 = create_and_add_char(&game_input_channel, game_id).await;
        let character2 = create_and_add_char(&game_input_channel, game_id).await;
        let character3 = create_and_add_char(&game_input_channel, game_id).await;
        let character4 = create_and_add_char(&game_input_channel, game_id).await;
        let combatants = vec![character1, character2, character3, character4];

        let msg = RequestMessage::StartCombat
            (CombatSetup{reply_channel: game_sender, game_id, combatants: combatants.clone()});

        assert!(game_input_channel.send(msg).await.is_ok());

        let (game_sender, _game_receiver) = channel::<ResponseMessage>();
        let msg = RequestMessage::BeginInitiativePhase(SimpleMessage{game_id, reply_channel: game_sender});
        assert!(game_input_channel.send(msg).await.is_ok());

        for character_id in combatants
        {
            let (game_sender, game_receiver) = channel::<ResponseMessage>();
            let roll: Roll = Roll{ reply_channel: game_sender, game_id, character_id, roll: 13 };
            let msg = RequestMessage::AddInitiativeRoll(roll);
            assert!(game_input_channel.send(msg).await.is_ok());
    
            match game_receiver.await
            {
                Ok(response) => {
                    match response
                    {
                        ResponseMessage::InitiativeRollAdded => {},
                        _ => {
                            panic!("Unexpected ResponseMessage - should have been InitiativeRollAdded.")
                        }
                    }
                },
                Err(_) => {
                    panic!("The oneshot channel errored out before the GameRunner could send a response.");
                } 
            } 
        }
    }

    async fn construct_combat_ready_game() -> (Sender<RequestMessage>, Uuid, Vec<Uuid>)
    {
        let game_input_channel = init();
        let (game_sender, _game_receiver) = channel::<ResponseMessage>();

        let game_id = add_new_game(&game_input_channel).await;

        let character1 = create_and_add_char(&game_input_channel, game_id).await;
        let character2 = create_and_add_char(&game_input_channel, game_id).await;
        let character3 = create_and_add_char(&game_input_channel, game_id).await;
        let character4 = create_and_add_char(&game_input_channel, game_id).await;
        let combatants = vec![character1, character2, character3, character4];

        let msg = RequestMessage::StartCombat
            (CombatSetup{reply_channel: game_sender, game_id, combatants: combatants.clone()});

        assert!(game_input_channel.send(msg).await.is_ok());

        let (game_sender, _game_receiver) = channel::<ResponseMessage>();
        let msg = RequestMessage::BeginInitiativePhase(SimpleMessage{game_id, reply_channel: game_sender});
        assert!(game_input_channel.send(msg).await.is_ok());

        return (game_input_channel, game_id, combatants);
    }

    #[tokio::test]
    pub async fn sending_start_combat_round_before_all_combatants_have_sent_initiatives_generates_invalid_state_action()
    {
        let (game_input_channel, game_id, combatants) = construct_combat_ready_game().await;

        let (game_sender, _game_receiver) = channel::<ResponseMessage>();
        let roll = Roll{ reply_channel: game_sender, game_id, character_id: *combatants.get(0).unwrap(), roll: 23 };
        assert!(game_input_channel.send(RequestMessage::AddInitiativeRoll(roll)).await.is_ok());
        
        let (game_sender, game_receiver) = channel::<ResponseMessage>();
        let msg = RequestMessage::StartCombatRound(SimpleMessage{ reply_channel: game_sender, game_id });

        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await
        {
            Ok(response) => 
            {
                match response
                {
                    ResponseMessage::Error(err) => 
                    {
                        assert!(err.kind == ErrorKind::InvalidStateAction);
                    },
                    _ => {panic!("Should have received an error, instead a non-error message was returned.")}
                }
            },
            Err(_) => panic!("The receiver errored waiting for the game to respond."),
        }
    }


    #[tokio::test]
    pub async fn sending_start_combat_round_to_newly_created_game_generates_invalid_state_action()
    {
        // let (game_input_channel, game_receiver) = tokio::sync::mpsc::channel(10);
        let game_input_channel = init();
        let game_id: Uuid;
        let (game_sender, game_receiver) = channel();
        let msg = RequestMessage::New(NewGame{ reply_channel: game_sender });

        assert!(game_input_channel.send(msg).await.is_ok());
        match game_receiver.await
        {
            Ok(response) => 
            {
                match response
                {
                    ResponseMessage::Created(id) => {game_id = id},
                    _ => {panic!("Failure creating game.")}
                }
            },
            Err(_) => panic!("Receiver errored waiting for game creation."),
        }

        let (game_sender, game_receiver) = channel();
        let msg = RequestMessage::StartCombatRound(SimpleMessage{ reply_channel: game_sender, game_id });

        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await
        {
            Ok(response) => 
            {
                match response
                {
                    ResponseMessage::Error(err) => 
                    {
                        assert!(err.kind == ErrorKind::InvalidStateAction);
                    },
                    _ => {panic!("Non-error response returned.");}
                }
            },
            Err(_) => panic!(),
        }

    }

    #[tokio::test]
    pub async fn sending_begin_initiative_after_declaring_combat_generates_invalid_state_action()
    {
        let (game_input_channel, game_id, combatants) = construct_combat_ready_game().await;

        let (game_sender, game_receiver) = channel::<ResponseMessage>();
        let msg = RequestMessage::StartCombatRound(SimpleMessage{ reply_channel: game_sender, game_id });

        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await
        {
            Ok(response) => 
            {
                match response
                {
                    ResponseMessage::Error(err) => 
                    {
                        assert!(err.kind == ErrorKind::InvalidStateAction);
                    },
                    _ => {panic!("Non-error message as response.")}
                }
            },
            Err(_) => {panic!("One shot channel panicked awaiting message.");},
        }
    }
    
    #[tokio::test]
    pub async fn begin_initiative_message_will_only_be_accepted_if_game_in_combat_phase_with_registered_combatants_or_action_round_ended()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<ResponseMessage>();
        let msg = RequestMessage::New(NewGame{ reply_channel: game_sender });

        assert!(game_input_channel.send(msg).await.is_ok());
        let game_id: Uuid;
        if let ResponseMessage::Created(generated_id) = game_receiver.await.unwrap()
        {
            game_id = generated_id;
        }
        else
        {
            panic!("New game failed to generate an ID.");
        }

        let character1 = create_and_add_char(&game_input_channel, game_id).await;
        let character2 = create_and_add_char(&game_input_channel, game_id).await;

        let (game_sender, game_receiver) = channel::<ResponseMessage>();
        let msg = RequestMessage::BeginInitiativePhase(SimpleMessage{ reply_channel: game_sender, game_id });
        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await
        {
            Ok(response) => 
            {
                match response
                {
                    ResponseMessage::Error(err) => {assert!(err.kind == ErrorKind::InvalidStateAction)},
                    _ => {panic!("Sending begin initiative to unprepared new game should generate error.")}
                }
            },
            Err(_) => {panic!("Message channel for game runner replies has errored out with no reply.")},
        }

        let (game_sender, game_receiver) = channel::<ResponseMessage>();
        let msg = RequestMessage::StartCombat(CombatSetup{ reply_channel: game_sender, game_id, combatants: vec![character1, character2] });
        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        let (game_sender, game_receiver) = channel::<ResponseMessage>();
        let msg = RequestMessage::BeginInitiativePhase(SimpleMessage { reply_channel: game_sender, game_id });
        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await
        {
            Ok(response) =>
            {
                match response
                {
                    ResponseMessage::InitiativePhaseStarted => {}
                    // ResponseMessage::Error(err) => {assert!(err.kind == ErrorKind::InvalidStateAction)}
                    _ => {panic!("Sending begin initiative round once combat phase started should produce an InitiativePhaseStarted response.")}
                }
            },
            Err(_) => {panic!("Message channel for game runner replies has errored out with no reply.")}
        }

        // let (game_sender, game_receiver) = channel::<ResponseMessage>();
        // let msg = RequestMessage::BeginInitiativePhase(SimpleMessage { reply_channel: game_sender, game_id });
        // assert!(game_input_channel.send(msg).await.is_ok());

        // match game_receiver.await
        // {
        //     Ok(response) =>
        //     {
        //         match response
        //         {
        //             ResponseMessage::Error(err) => {assert!(err.kind == ErrorKind::InvalidStateAction)}
        //             _ => {panic!("Sending begin initiative round once combat phase started should produce an InitiativePhaseStarted response.")}
        //         }
        //     },
        //     Err(_) => {panic!("Message channel for game runner replies has errored out with no reply.")}
        // }

    }
}