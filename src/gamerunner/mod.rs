use log::{debug, error};
use tokio::sync::mpsc::{Receiver, Sender as MpscSender};
use uuid::Uuid;

use crate::gamerunner::{registry::GameRegistry, authority::authorize};
use notifier::{into_notification, notify_players, WhatChanged};
use dispatcher::dispatch_message;

use self::dispatcher::Message;

pub mod registry;
pub mod authority;
pub mod dispatcher;
pub mod notifier;

pub async fn game_runner(mut message_queue: Receiver<Message>)
{
    debug!("Game runner redux started.");

    let mut directory = GameRegistry::new();

    while let Some(message) = message_queue.recv().await
    {
        let (channel, player_id_opt, game_id_opt, request) = 
            (message.reply_channel, message.player_id, message.player_id, message.msg);

        let authority = authorize(player_id_opt, game_id_opt, request, &directory);
        // let (channel, game_id) = (message.reply_channel, message.game_id);
        let (response, to_notify) = dispatch_message(&mut directory, &authority);

        // if let Some(notification) = into_notification(&directory,&response, &game_id, todo!())
        // {
        //     if let Some(players) = to_notify
        //     {
        //         let senders: Vec<MpscSender<WhatChanged>> = players.iter()
        //             .map(|f| directory.get_player_sender(f).clone())
        //             .filter(|o| o.is_some()).map(|o| o.unwrap()).collect();
        //         notify_players(notification, &senders).await;
        //     }
        //     else
        //     {
        //         if let Some(players) = directory.players_by_game(&game_id)
        //         {
        //             let senders: Vec<MpscSender<WhatChanged>> = players.iter().map(|f| directory.get_player_sender(f).clone())
        //                 .filter(|o| o.is_some()).map(|o| o.unwrap()).collect();
        //             notify_players(notification, &senders).await;
        //         }
        //     }
        // }

        if channel.send(response).is_err()
        {
            error!("The return channel has dropped.");
        }
    }
}


type PlayerId = Uuid;
type GameId = Uuid;
type CharacterId = Uuid;

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
    NotGameOwner,
    NotGamePlayer,
    UnknownPlayerId,
    NoMatchingGame,
    NoSuchCharacter,
    InvalidStateAction,
    CannotAdvanceTurn,
    NoActionLeft,
    NotCharactersTurn,
    NoEventsLeft,
    UnresolvedCombatant,
}

#[cfg(test)]
mod tests
{
    use core::panic;


    use log::debug;
    use tokio::sync::oneshot::Receiver;
    use tokio::sync::oneshot::channel;
    use tokio::sync::mpsc::channel as mpsc_channel;
    use tokio::sync::mpsc::Sender;
    use uuid::Uuid;
    

    use crate::gamerunner::dispatcher::Action;
    use crate::gamerunner::{game_runner, dispatcher::{Outcome, Request}};
    use crate::tracker::character::Character;
    use crate::tracker::character::Metatypes;
    use crate::tracker::game::ActionType;
    use crate::gamerunner::WhatChanged;

    use super::ErrorKind;
    use super::Message;
    use super::dispatcher::NewPlayer;
    use super::dispatcher::Roll;

    pub fn init() -> Sender<Message> {
        let _ = env_logger::builder().is_test(true).try_init();
        debug!("Logger should be active.");

        debug!("Created multi-producer, single consumer channel");
        let (sender, receiver) = mpsc_channel(1);

        debug!("About to start game runner.");
        tokio::spawn(async {game_runner(receiver).await;});

        debug!("Runner started, returning.");
        return sender;
    }

    pub async fn add_new_game(game_input_channel: &Sender<Message>) -> Uuid
    {
        let (game_sender, game_receiver) = channel();
        let msg = Message { player_id: None, game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::New };

        match game_input_channel.send(msg).await {
            Ok(_) => {
                match game_receiver.await
                {
                    Ok(game_msg) => {
                        match game_msg
                        {
                            Outcome::Created(id) => {return id},
                            _ => {panic!("Received a ResponseMessage enum of an unexpected type.")}
                        }
                    },
                    Err(_) => panic!{"The oneshot channel closed while waiting for reply."},
                }
            },
            Err(_) => panic!("Game input channel closed while waiting for reply."),
        }
    }

    pub async fn player_join_game(game_input_channel: &Sender<Message>, game_id: Uuid) -> NewPlayer
    {
        let (game_sender, game_receiver) = channel();
        
        let msg = Message {player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::NewPlayer};

        if let Err(_) = game_input_channel.send(msg).await 
        {
            panic!("The game runner input channel closed prematurely.");
        };

        return match game_receiver.await
        {
            Ok(from_game) =>
            {
                match from_game
                {
                    Outcome::NewPlayer(player_state) => player_state,
                    _ => panic!("Was expecting NewPlayer registration confirmation.")
                }
            }
            Err(_) => panic!("Game input channel has closed.")
        }
    }

    #[tokio::test]
    pub async fn if_a_person_wishes_to_play_they_must_register_as_a_player_to_get_a_player_id()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel();
        let game_id = add_new_game(&game_input_channel).await;

        let msg = Message {player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::NewPlayer};

        if let Err(_) = game_input_channel.send(msg).await 
        {
            panic!("The game runner input channel closed prematurely.");
        };

        match game_receiver.await
        {
            Ok(from_game) =>
            {
                match from_game
                {
                    Outcome::NewPlayer(_player_state) => {}
                    _ => panic!("Was expecting NewPlayer registration confirmation.")
                }
            }
            Err(_) => panic!("Game input channel has closed.")
        }
    }

    #[tokio::test]
    pub async fn when_a_new_player_joins_a_game_they_receive_a_game_state_return_value()
    {
        let game_input_channel = init();
        let game_id = add_new_game(&game_input_channel).await;

        let player_state = player_join_game(&game_input_channel, game_id).await;

        let (game_sender, game_receiver) = channel();
        let msg = Message {player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame(player_state.player_id)};

        if let Err(_) = game_input_channel.send(msg).await 
        {
            panic!("The game runner input channel closed prematurely.");
        };

        match game_receiver.await 
        {
            Ok(outcome) =>
            {
                match outcome 
                {
                    Outcome::JoinedGame(_) => 
                    {
                        // This is an acceptable return state.
                    }
                    _ => {panic!("Received an unexpected response - should have been JoinedGame.")}
                }
            }
            Err(_) => panic!("The GameRunner should have returned a current GameState object along with my update messaging channel.")
        }
    }

    #[tokio::test]
    pub async fn when_a_new_player_joins_a_game_existing_players_receive_a_notification()
    {
        init();

        let game_input_channel = init();
        let game_id = add_new_game(&game_input_channel).await;

        let NewPlayer {player_id: player_1_id, player_1_receiver: mut player_1_channel} 
            = player_join_game(&game_input_channel, game_id).await;
        let (mut game_sender, mut game_receiver) = channel();
        let mut msg = Message {player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame(player_1_id)};

        assert!(game_input_channel.send(msg).await.is_ok() );
        assert!(game_receiver.await.is_ok());

        let player_state = player_join_game(&game_input_channel, game_id).await;
        (game_sender, game_receiver) = channel();
        msg = Message {player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame(player_state.player_id)};

        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        match player_1_channel.recv().await 
        {
            Some(msg) =>
            {
                if let WhatChanged::NewPlayer(_name) = msg {}
                else {panic!("Wrong message type for notification.")}
            }
            None => {panic!("Should have received a WhatsChanged message.")}
        }


    }


    pub fn create_character() -> Character
    {
        let names: [&str; 5] = ["Matrox", "El See-Dee", "BusShock", "Junkyard", "Lo Hax"];
        let metatypes = [Metatypes::Dwarf, Metatypes::Elf, Metatypes::Human, Metatypes::Orc, Metatypes::Troll];

        if rand::random::<usize>() % 2 == 1 {
            return Character::new_npc(metatypes[rand::random::<usize>() % 5], String::from(names[rand::random::<usize>() % 5]));
        }

        return Character::new_pc(metatypes[rand::random::<usize>() % 5], String::from(names[rand::random::<usize>() % 5]));
        
    }

    async fn create_and_add_char(game_input_channel: &Sender<Message>, game_id: Uuid) -> Uuid
    {
        let (game_sender, game_receiver) = channel::<Outcome>();

        let character = create_character();

        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::AddCharacter(character) };
        let send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());

        let response = game_receiver.await;
        assert!(response.is_ok());

        match response
        {
            Ok(msg) => {
                match msg
                {
                    Outcome::CharacterAdded((game_id, character_id)) => {return character_id;}
                    _ => {panic!("Attempt to add character for test failed.");}
                }
            },
            Err(_) => {panic!("Channel closed.")}
        }
    }

    

    #[tokio::test]
    pub async fn enumerating_games_before_creating_a_game_will_return_an_empty_list()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel();

        let msg = Message{ player_id: None, game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::Enumerate };
        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await
        {
            Ok(outcome) => 
            {
                match outcome
                {
                    Outcome::Summaries(summaries) => 
                    {
                        assert!(summaries.len() == 0);
                    },
                    _ => { panic!("Should have recieved an Outcome::Summaries with an empty vec.")}
                }
            },
            Err(_) => {panic!("The oneshot receiver channel terminated unexpectedly!")},
        }
    }

    #[tokio::test]
    pub async fn enumerating_games_after_creating_games_returns_non_empty_vec()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel();

        let msg = Message{ player_id: None, game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::New };
        assert!(game_input_channel.send(msg).await.is_ok());

        let id: Uuid;

        if let Ok(outcome) = game_receiver.await
        {
            match outcome 
            {
                Outcome::Created(game_id) => { id = game_id },
                _ => { panic!("Should have been a created message.")}
            }
        }
        else { panic!("game_receiver errored out."); }

        let (game_sender, game_receiver) = channel();

        let msg = Message{ player_id: None, game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::Enumerate };
        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await
        {
            Ok(outcome) => 
            {
                match outcome
                {
                    Outcome::Summaries(summaries) => 
                    {
                        assert!(summaries.len() == 1);
                        assert!(summaries.get(0).unwrap().0 == id);
                    },
                    _ => { panic!("Should have recieved an Outcome::Summaries with an empty vec.")}
                }
            },
            Err(_) => {panic!("The oneshot receiver channel terminated unexpectedly!")},
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
        let msg = Message{ player_id: None, game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::New };

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
            Outcome::Created(_uuid) => {
                
            },
            _ => {panic!("No other type should have been possible.")}
        }
    }

    #[tokio::test]
    pub async fn deleting_a_game_with_its_id_will_generate_destroyed_message()
    {
        let game_input_channel = init();

        let (game_sender, game_receiver) = channel::<Outcome>();

        // when I send a Delete message with one half of a oneshot channel and a game ID that really exists...
        let game_id = add_new_game(&game_input_channel).await;

        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::Delete };
        
        let send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());

        let response = game_receiver.await;
        assert!(response.is_ok());

        match response.unwrap()
        {
            Outcome::Destroyed => {/* This is good, nothing to do. */},
            Outcome::Error(err) => {panic!("Received an error: {}", err.message);}
            _ => {panic!("Received ResponseMessage that should not have been generated by request.");}
        }
    }

    #[tokio::test]
    pub async fn when_a_game_is_deleted_it_will_notify_all_current_players_of_the_event()
    {
        let game_input_channel = init();

        let (mut game_sender, mut game_receiver) = channel::<Outcome>();

        let game_id = add_new_game(&game_input_channel).await;

        let mut msg = Message {player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::NewPlayer};
        let mut send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());
        let (melf_id, mut melf_notifications) = match game_receiver.await.unwrap()
        {
            Outcome::NewPlayer(player_struct) => (player_struct.player_id, player_struct.player_1_receiver),
            _ => {panic!("These match arms should not have been invoked.")}
        };

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message {player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::NewPlayer};
        send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());
        let (mork_id, mut mork_notifications) = match game_receiver.await.unwrap()
        {
            Outcome::NewPlayer(player_struct) => (player_struct.player_id, player_struct.player_1_receiver),
            _ => {panic!("These match arms should not have been invoked.")}
        };

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message {player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame(mork_id)};
        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message {player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame(melf_id)};
        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        (game_sender, game_receiver) = channel::<Outcome>();

        msg = Message {player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::Delete};
        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        
        loop
        {
            let change_option = mork_notifications.try_recv();
            match change_option
            {
                Ok(change_notice) => 
                {
                    match change_notice
                    {
                        WhatChanged::GameEnded => {break;},
                        _ => {}
                    }
                },
                Err(err) =>
                {
                    match err
                    {
                        tokio::sync::mpsc::error::TryRecvError::Empty => 
                        {
                            panic!("Channel emptied out without ever giving up the GameEnded message");
                        },
                        tokio::sync::mpsc::error::TryRecvError::Disconnected => 
                        {
                            panic!("Channel closed without ever giving up the GameEnded message");
                        },
                    }
                }
            }
        }

        loop
        {
            let change_option = melf_notifications.try_recv();
            match change_option
            {
                Ok(change_notice) =>
                {
                    match change_notice
                    {
                        WhatChanged::GameEnded => {break;},
                        _ => {}
                    }
                },
                Err(err) =>
                {
                    match err
                    {
                        tokio::sync::mpsc::error::TryRecvError::Empty => 
                        {
                            panic!("Melf channel emptied out wihtout ever giving up the GameEnded message");
                        },
                        tokio::sync::mpsc::error::TryRecvError::Disconnected =>
                        {
                            panic!("Melf channel closed without ever giving up the GameEnded message");
                        }
                    }
                }
            }
        }

        // match melf_notifications.recv().await
        // {
        //     Some(change_notice) =>  match change_notice {
        //         WhatChanged::GameEnded => {},
        //         _ => {panic!("Should have received game ended notification")}
        //     },
        //     None => { panic!("Should have produced a WhatChanged notification.")}
        // }
    }


    #[tokio::test]
    pub async fn deleting_a_game_with_an_unknown_id_will_generate_no_matching_game()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<Outcome>();

        add_new_game(&game_input_channel).await;

        let msg = Message { player_id: None, game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::Delete };
        
        let send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());

        let response = game_receiver.await;
        assert!(response.is_ok());

        match response.unwrap()
        {
            Outcome::Destroyed => {panic!("The game deleted somehow - received a Destroyed message instead of an error.");},
            Outcome::Error(err) => 
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
        let (game_sender, game_receiver) = channel::<Outcome>();

        let game_id = add_new_game(&game_input_channel).await;

        let character = create_character();

        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::AddCharacter(character) };
        let send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());

        let response = game_receiver.await;
        assert!(response.is_ok());

        match response.unwrap()
        {
            Outcome::Error(_) => {panic!("This should have been a successful add.")},
            Outcome::CharacterAdded(_) => {},
            _ => {panic!("Another message was triggered, but add new character should result only in error or character added.")}
        }
    }

    #[tokio::test]
    pub async fn adding_a_character_to_a_non_extant_game_will_generate_no_matching_game()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<Outcome>();

        let _ = add_new_game(&game_input_channel).await;

        let character = create_character();

        let msg = Message { player_id: None, game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::AddCharacter(character) };
        let send_state = game_input_channel.send(msg).await;

        assert!(send_state.is_ok());

        let response = game_receiver.await;
        assert!(response.is_ok());

        match response.unwrap()
        {
            Outcome::Error(err) => 
            { 
                assert!(err.kind == ErrorKind::NoMatchingGame);
            },
            Outcome::CharacterAdded(_) => {panic!("This add should have failed - should have received Error rather than CharacterAdded.")},
            _ => {panic!("Another message was triggered, but add new character should result only in error or character added.")}
        }
    }

    #[tokio::test]
    pub async fn starting_combat_with_registered_characters_will_generate_combat_started_message()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<Outcome>();

        let game_id = add_new_game(&game_input_channel).await;

        let character1 = create_and_add_char(&game_input_channel, game_id).await;
        let character2 = create_and_add_char(&game_input_channel, game_id).await;
        let character3 = create_and_add_char(&game_input_channel, game_id).await;
        let character4 = create_and_add_char(&game_input_channel, game_id).await;

        let combatants = vec![character1, character2, character3, character4];

        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(combatants) };

        let response = game_input_channel.send(msg).await;

        assert!(response.is_ok());
        
        match game_receiver.await
        {
            Ok(msg) => {
                match msg {
                    Outcome::CombatStarted => {} // Success, nothing in the response to test.
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
        let (game_sender, game_receiver) = channel::<Outcome>();

        let game_id = add_new_game(&game_input_channel).await;

        let _character1 = create_and_add_char(&game_input_channel, game_id).await;
        let _character2 = create_and_add_char(&game_input_channel, game_id).await;
        let _character3 = create_and_add_char(&game_input_channel, game_id).await;
        let _character4 = create_and_add_char(&game_input_channel, game_id).await;

        let combatants = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];

        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(combatants) };

        let response = game_input_channel.send(msg).await;

        assert!(response.is_ok());
        
        match game_receiver.await
        {
            Ok(msg) => {
                match msg {
                    Outcome::CombatStarted => {panic!("The combat stage was started, but the Ids provided should not be characters.");}
                    Outcome::Error(err) => {
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
        let (game_sender, _game_receiver) = channel::<Outcome>();

        let game_id = add_new_game(&game_input_channel).await;

        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(Vec::<Uuid>::new()) };

        let response = game_input_channel.send(msg).await;

        assert!(response.is_ok()); // It is entirely acceptable to start a combat with no combatants.  Individual combatants can be added later,
        // or another batch of combatants can be added later.
    }

    #[tokio::test]
    pub async fn starting_combat_with_an_unregistered_game_id_will_generate_no_matching_game()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<Outcome>();

        let game_id = add_new_game(&game_input_channel).await;

        let character1 = create_and_add_char(&game_input_channel, game_id).await;
        let character2 = create_and_add_char(&game_input_channel, game_id).await;
        let character3 = create_and_add_char(&game_input_channel, game_id).await;
        let character4 = create_and_add_char(&game_input_channel, game_id).await;

        let combatants = vec![character1, character2, character3, character4];

        let msg = Message { player_id: None, game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::StartCombat(combatants) };

        let response = game_input_channel.send(msg).await;

        assert!(response.is_ok());
        
        match game_receiver.await
        {
            Ok(msg) => {
                match msg {
                    Outcome::CombatStarted => {panic!("This should have returned an error!");}
                    Outcome::Error(err) => {
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
        let (game_sender, _game_receiver) = channel::<Outcome>();

        let game_id = add_new_game(&game_input_channel).await;

        let character1 = create_and_add_char(&game_input_channel, game_id).await;
        let character2 = create_and_add_char(&game_input_channel, game_id).await;
        let character3 = create_and_add_char(&game_input_channel, game_id).await;
        let character4 = create_and_add_char(&game_input_channel, game_id).await;
        let combatants = vec![character1, character2, character3, character4];

        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(combatants) };

        let _response = game_input_channel.send(msg).await;

        let (game_sender, game_receiver) = channel::<Outcome>();

        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase };

        let response = game_input_channel.send(msg).await;

        assert!(response.is_ok());

        match game_receiver.await
        {
            Ok(msg) => {
                match msg
                {
                    Outcome::InitiativePhaseStarted => {} // all is good
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
        let (game_sender, _game_receiver ) = channel::<Outcome>();

        let game_id = add_new_game(&game_input_channel).await;

        let _character1 = create_and_add_char(&game_input_channel, game_id).await;
        let _character2 = create_and_add_char(&game_input_channel, game_id).await;
        let _character3 = create_and_add_char(&game_input_channel, game_id).await;

        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(Vec::<Uuid>::new()) };

        let _response = game_input_channel.send(msg).await;

        let (game_sender, game_receiver) = channel::<Outcome>();

        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase };

        let response = game_input_channel.send(msg).await;

        assert!(response.is_ok());

        match game_receiver.await
        {
            Ok(msg) => {
                match msg
                {
                    Outcome::Error(kind) => {
                        if kind.kind != ErrorKind::NoSuchCharacter
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
        let (game_sender, _game_receiver) = channel::<Outcome>();

        let game_id = add_new_game(&game_input_channel).await;

        let character1 = create_and_add_char(&game_input_channel, game_id).await;
        let character2 = create_and_add_char(&game_input_channel, game_id).await;

        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(vec![character1, character2]) };

        assert!(game_input_channel.send(msg).await.is_ok());

        let (game_sender, _game_receiver) = channel::<Outcome>();
        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase };
        assert!(game_input_channel.send(msg).await.is_ok());

        let (game_sender, game_receiver) = channel::<Outcome>();
        let roll: Roll = Roll{ character_id: character1, roll: 13 };
        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::AddInitiativeRoll(roll) };
        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await
        {
            Ok(response) => {
                match response
                {
                    Outcome::InitiativeRollAdded => {},
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
        let (game_sender, _game_receiver) = channel::<Outcome>();

        let game_id = add_new_game(&game_input_channel).await;

        let character1 = create_and_add_char(&game_input_channel, game_id).await;
        let character2 = create_and_add_char(&game_input_channel, game_id).await;
        let character3 = create_and_add_char(&game_input_channel, game_id).await;
        let character4 = create_and_add_char(&game_input_channel, game_id).await;
        let combatants = vec![character1, character2, character3, character4];

        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(combatants.clone()) };

        assert!(game_input_channel.send(msg).await.is_ok());

        let (game_sender, _game_receiver) = channel::<Outcome>();
        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase };
        assert!(game_input_channel.send(msg).await.is_ok());

        for character_id in combatants
        {
            let (game_sender, game_receiver) = channel::<Outcome>();
            let roll: Roll = Roll{character_id, roll: 13 };
            let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::AddInitiativeRoll(roll) };
            assert!(game_input_channel.send(msg).await.is_ok());
    
            match game_receiver.await
            {
                Ok(response) => {
                    match response
                    {
                        Outcome::InitiativeRollAdded => {},
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

    async fn construct_combat_ready_game() -> (Sender<Message>, Uuid, Vec<Uuid>)
    {
        let game_input_channel = init();
        let (game_sender, _game_receiver) = channel::<Outcome>();

        let game_id = add_new_game(&game_input_channel).await;

        let character1 = create_and_add_char(&game_input_channel, game_id).await;
        let character2 = create_and_add_char(&game_input_channel, game_id).await;
        let character3 = create_and_add_char(&game_input_channel, game_id).await;
        let character4 = create_and_add_char(&game_input_channel, game_id).await;
        let combatants = vec![character1, character2, character3, character4];

        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(combatants.clone()) };

        assert!(game_input_channel.send(msg).await.is_ok());

        let (game_sender, _game_receiver) = channel::<Outcome>();
        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase };
        assert!(game_input_channel.send(msg).await.is_ok());

        return (game_input_channel, game_id, combatants);
    }

    #[tokio::test]
    pub async fn sending_start_combat_round_before_all_combatants_have_sent_initiatives_generates_invalid_state_action()
    {
        let (game_input_channel, game_id, combatants) = construct_combat_ready_game().await;

        let (game_sender, _game_receiver) = channel::<Outcome>();
        let roll = Roll{ character_id: *combatants.get(0).unwrap(), roll: 23 };
        let msg = Message{player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::AddInitiativeRoll(roll)};
        assert!(game_input_channel.send(msg).await.is_ok());
        
        let (game_sender, game_receiver) = channel::<Outcome>();
        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombatRound };

        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await
        {
            Ok(response) => 
            {
                match response
                {
                    Outcome::Error(err) => 
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
        let game_input_channel = init();
        let game_id: Uuid;
        let (game_sender, game_receiver) = channel();
        let msg = Message { player_id: None, game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::New };

        assert!(game_input_channel.send(msg).await.is_ok());
        match game_receiver.await
        {
            Ok(response) => 
            {
                match response
                {
                    Outcome::Created(id) => {game_id = id},
                    _ => {panic!("Failure creating game.")}
                }
            },
            Err(_) => panic!("Receiver errored waiting for game creation."),
        }

        let (game_sender, game_receiver) = channel();
        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombatRound };

        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await
        {
            Ok(response) => 
            {
                match response
                {
                    Outcome::Error(err) => 
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
        let (game_input_channel, game_id, _combatants) = construct_combat_ready_game().await;

        let (game_sender, game_receiver) = channel::<Outcome>();
        let msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombatRound };

        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await
        {
            Ok(response) => 
            {
                match response
                {
                    Outcome::Error(err) => 
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
        let (mut game_sender, mut game_receiver) = channel::<Outcome>();
        let mut _game_receiver: Receiver<Outcome>;
        let mut msg = Message { player_id: None, game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::New };

        assert!(game_input_channel.send(msg).await.is_ok());
        let game_id: Uuid;
        if let Outcome::Created(generated_id) = game_receiver.await.unwrap()
        {
            game_id = generated_id;
        }
        else
        {
            panic!("New game failed to generate an ID.");
        }

        let character1 = create_and_add_char(&game_input_channel, game_id).await;
        let character2 = create_and_add_char(&game_input_channel, game_id).await;

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase };
        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await
        {
            Ok(response) => 
            {
                match response
                {
                    Outcome::Error(err) => {assert!(err.kind == ErrorKind::NoSuchCharacter)},
                    _ => {panic!("Sending begin initiative to unprepared new game should generate error.")}
                }
            },
            Err(_) => {panic!("Message channel for game runner replies has errored out with no reply.")},
        }

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(vec![character1, character2]) };
        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        (game_sender, game_receiver) = channel::<Outcome>();

        msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase };
        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await
        {
            Ok(response) =>
            {
                match response
                {
                    Outcome::InitiativePhaseStarted => {}
                    // ResponseMessage::Error(err) => {assert!(err.kind == ErrorKind::InvalidStateAction)}
                    _ => {panic!("Sending begin initiative round once combat phase started should produce an InitiativePhaseStarted response.")}
                }
            },
            Err(_) => {panic!("Message channel for game runner replies has errored out with no reply.")}
        }

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message{player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase};
        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await
        {
            Ok(response) =>
            {
                match response
                {
                    Outcome::Error(err) => {assert!(err.kind == ErrorKind::InvalidStateAction)}
                    _ => {panic!("Sending begin initiative round once combat phase started should produce an InitiativePhaseStarted response.")}
                }
            },
            Err(_) => {panic!("Message channel for game runner replies has errored out with no reply.")}
        }

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::AddInitiativeRoll(Roll { character_id: character1, roll: 13 })};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::AddInitiativeRoll(Roll { character_id: character2, roll: 23 })};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombatRound};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::TakeAction(Action { character_id: character2, action: ActionType::Complex })};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::AdvanceTurn};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::TakeAction(Action { character_id: character1, action: ActionType::Complex })};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::AdvanceTurn};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message{player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase};
        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await
        {
            Ok(response) =>
            {
                match response
                {
                    Outcome::InitiativePhaseStarted => {}
                    _ => {panic!("Sending begin initiative round once combat phase started should produce an InitiativePhaseStarted response.")}
                }
            },
            Err(_) => {panic!("Message channel for game runner replies has errored out with no reply.")}
        }

    }

    #[tokio::test]
    pub async fn when_the_highest_initiative_player_acts_in_combat_the_outcome_should_be_action_taken()
    {
        let (sender, game_id, characters) = construct_combat_ready_game().await;

        let (mut game_owned_sender, mut our_receiver) = channel::<Outcome>();
        let mut msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *characters.get(0).unwrap(), roll: 13 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *characters.get(1).unwrap(), roll: 23 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *characters.get(2).unwrap(), roll: 9 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *characters.get(3).unwrap(), roll: 16 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());

        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::StartCombatRound};
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::TakeAction
            (Action{character_id: *characters.get(1).unwrap(), action: ActionType::Complex})};
        
        assert!(sender.send(msg).await.is_ok());

        match our_receiver.await
        {
            Ok(response) => 
            {
                match response
                {
                    Outcome::ActionTaken => {},
                    _ => {panic!("The outcome should have been ActionTaken.")}
                }
            },
            Err(_) => {panic!("Letting the highest initiative character take an action caused an error.")},
        }

        
    }

    #[tokio::test]
    pub async fn when_in_combat_rounds_any_character_can_use_their_free_action_anytime()
    {
        let (sender, game_id, characters) = construct_combat_ready_game().await;

        let (mut game_owned_sender, mut our_receiver) = channel::<Outcome>();
        let mut msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *characters.get(0).unwrap(), roll: 13 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *characters.get(1).unwrap(), roll: 23 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *characters.get(2).unwrap(), roll: 9 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *characters.get(3).unwrap(), roll: 16 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());

        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::StartCombatRound};
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());

        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::TakeAction(Action{ character_id: *characters.get(2).unwrap(), action: ActionType::Free })};
        assert!(sender.send(msg).await.is_ok());
        
        match our_receiver.await
        {
            Ok(response) => 
            {
                match response
                {
                    Outcome::ActionTaken => {},
                    _ => {panic!("The outcome should have been ActionTaken.")}
                }
            },
            Err(_) => {panic!("Letting the highest initiative character take an action caused an error.")},
        }
        
    }

    #[tokio::test]
    pub async fn a_character_that_takes_simple_or_complex_action_out_of_turn_will_generate_not_characters_turn_error()
    {
        let (sender, game_id, characters) = construct_combat_ready_game().await;

        let (mut game_owned_sender, mut our_receiver) = channel::<Outcome>();
        let mut msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *characters.get(0).unwrap(), roll: 13 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *characters.get(1).unwrap(), roll: 23 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *characters.get(2).unwrap(), roll: 9 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *characters.get(3).unwrap(), roll: 16 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());

        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::StartCombatRound};
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());

        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: None, game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::TakeAction
            (Action{ character_id: *characters.get(3).unwrap(), action: ActionType::Complex })};
        assert!(sender.send(msg).await.is_ok());

        match our_receiver.await
        {
            Ok(outcome) => 
            {
                match outcome
                {
                    Outcome::Error(err) => 
                    {
                        assert!(err.kind == ErrorKind::NotCharactersTurn)
                    },
                    _ => {panic!("The outcome should have been an error.");}
                }
            }
            Err(_) => {panic!("The one-shot receiver dropped.");},
        }
    }

    
}