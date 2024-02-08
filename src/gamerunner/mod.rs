use log::{debug, error};
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

use crate::gamerunner::{registry::GameRegistry, authority::authorize};
use notifier::{/*into_notification, notify_players,*/ WhatChanged};
use dispatcher::dispatch_message2;

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
            (message.reply_channel, message.player_id, message.game_id, message.msg);

        let mut_directory = &mut directory;
        let authority = authorize(player_id_opt, game_id_opt, request, mut_directory);
        // let (channel, game_id) = (message.reply_channel, message.game_id);
        let (response, notify_opt) = dispatch_message2(mut_directory, &authority);

        if let Some(notification) = notify_opt // = into_notification(&directory,&response, &authority)
        {
            let (message, sender_list) = (notification.change_type, notification.send_to);

            for sender in sender_list
            {
                // The sender's error variant is ignored.  If the send request errors out, that means that the recipient's channel has closed or 
                // broken, and we really cannot fix that.  Right now, we do not provide a way to establish a new channel - but even when we do, 
                // establishing a new channel will be at the discretion of the consumer.  We will just ignore the error and continue operating, 
                // at least until we make this more robust.
                let _ = sender.send(message.clone()).await;
                // TODO: err variant possible for with the above warning
            }
        }

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
    UnknownId,
    NoMatchingGame,
    NoSuchCharacter,
    InvalidStateAction,
    CannotAdvanceTurn,
    NoActionLeft,
    NotCharactersTurn,
    NoEventsLeft,
    UnresolvedCombatant, 
    UnauthorizedAction,
    Unexpected,
}

#[cfg(test)]
mod tests
{
    use core::panic;
    use std::collections::HashMap;


    use log::debug;
    use tokio::sync::oneshot::{Sender as OneShotSender, Receiver};
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

    use super::CharacterId;
    use super::ErrorKind;
    use super::GameId;
    use super::Message;
    use super::PlayerId;
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

    pub async fn add_new_game(game_input_channel: &Sender<Message>) -> (PlayerId, GameId)
    {
        debug!("Starting add_new_game");
        let gm: PlayerId;
        let gm_name = String::from("King Ghidorah");

        let (mut game_sender, mut game_receiver) = channel();
        let msg = Message { player_id: None, game_id: None, reply_channel: game_sender, msg: Request::NewPlayer(gm_name)};

        debug!("Message to register new player done and sending.");

        assert!(game_input_channel.send(msg).await.is_ok());
        gm = match game_receiver.await {
            Ok(Outcome::NewPlayer(player_id)) => player_id.player_id, 
            _ => panic!("Should have received a NewPlayer object with the id and messaging channel.")
        };

        debug!("Message to register new player has been sent and OK received from response channel.  Player id: {}", gm);

        (game_sender, game_receiver) = channel();
        let game_name = String::from("Spaaaace Madnessssss");
        let msg = Message { player_id: Some(gm), game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::NewGame(game_name) };

        debug!("Message to create new game done and about to send.");

        match game_input_channel.send(msg).await {
            Ok(_) => {
                match game_receiver.await
                {
                    Ok(Outcome::Created(id)) => {
                        debug!("Message has been accepted and new game {} has been created.", id);
                        return (gm, id)
                    },
                    Ok(_) | Err(_) => {
                        debug!("Message has been rejected.");
                        panic!{"The oneshot channel closed while waiting for reply."}
                    },
                }
            },
            Err(_) => panic!("Game input channel closed while waiting for reply."),
        }
    }

    pub async fn player_join_game(game_input_channel: &Sender<Message>, game_id: Uuid) -> NewPlayer
    {
        let (game_sender, game_receiver) = channel();
        let player_name = String::from("Lizard");
        
        let msg = Message {player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::NewPlayer(player_name)};

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
        let (_, game_id) = add_new_game(&game_input_channel).await;

        let player_name = String::from("Lizard");

        let msg = Message {player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::NewPlayer(player_name)};

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
        let (_, game_id) = add_new_game(&game_input_channel).await;

        let player_state = player_join_game(&game_input_channel, game_id).await;

        let (game_sender, game_receiver) = channel();
        let msg = Message {player_id: Some(player_state.player_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame};

        if let Err(_) = game_input_channel.send(msg).await 
        {
            panic!("The game runner input channel closed prematurely.");
        };

        match game_receiver.await 
        {
            Ok(Outcome::JoinedGame(_)) =>
            {

            },
            Ok(_) => panic!("Received an unexpected response - should have been JoinedGame."),
            Err(_) => panic!("The GameRunner should have returned a current GameState object along with my update messaging channel.")
        }
    }

    #[tokio::test]
    pub async fn when_a_new_player_joins_a_game_existing_players_receive_a_notification()
    {
        init();

        let game_input_channel = init();
        let (_, game_id) = add_new_game(&game_input_channel).await;

        let NewPlayer {player_id: player_1_id, player_1_receiver: mut player_1_channel} 
            = player_join_game(&game_input_channel, game_id).await;
        let (mut game_sender, mut game_receiver) = channel();
        let mut msg = Message {player_id: Some(player_1_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame};

        assert!(game_input_channel.send(msg).await.is_ok() );
        assert!(game_receiver.await.is_ok());

        let player_state = player_join_game(&game_input_channel, game_id).await;
        (game_sender, game_receiver) = channel();
        msg = Message {player_id: Some(player_state.player_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame};

        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        match player_1_channel.recv().await 
        {
            Some(msg) =>
            {
                if let WhatChanged::NewPlayer(_name) = msg.as_ref() {}
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

    async fn create_and_add_char(game_input_channel: &Sender<Message>, game_id: Uuid) -> (PlayerId, CharacterId)
    {
        debug!("Starting create_and_add_char()");

        let mut game_sender: OneShotSender<Outcome>;
        let mut game_receiver: Receiver<Outcome>;

        (game_sender, game_receiver) = channel::<Outcome>();
        let player_name = String::from("Lizard");
        
        debug!("Adding a player.");
        let mut msg = Message {player_id: None, game_id: None, reply_channel: game_sender, msg: Request::NewPlayer(player_name)};
        assert!(game_input_channel.send(msg).await.is_ok());

        let player_id = match game_receiver.await {
            Ok(Outcome::NewPlayer(player)) => player.player_id, 
            _ => panic!("Attempt to create new player has failed.")
        };

        debug!("Player {} added.", player_id);
        debug!("Player sending request to join game {}", game_id);

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message {player_id: Some(player_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame};
        assert!(game_input_channel.send(msg).await.is_ok());

        match game_receiver.await {
            Ok(Outcome::JoinedGame(state)) => {assert_eq!(player_id, state.for_player)}
            _ => panic!("Attempt to join game failed.")
        }

        (game_sender, game_receiver) = channel::<Outcome>();
        let character = create_character();

        msg = Message { player_id: Some(player_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::AddCharacter(character) };
        let send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());

        let response = game_receiver.await;
        assert!(response.is_ok());

        match response
        {
            Ok(Outcome::CharacterAdded((_, character_id))) => {
                    return (player_id, character_id);
            },
            Ok(_) => {panic!("Should have received CharacterAdded outcome - interface changed.")}
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
        let (mut game_sender, mut game_receiver) = channel();
        let player_name = String::from("Lizard");

        let mut msg = Message {player_id: None, game_id: None, reply_channel: game_sender, msg: Request::NewPlayer(player_name)};
        assert!(game_input_channel.send(msg).await.is_ok());
        let player_id = match game_receiver.await {
            Ok(Outcome::NewPlayer(player_obj)) => player_obj.player_id,
            _ => panic!("Expected NewPlayer message.")
        };

        (game_sender, game_receiver) = channel();
        let game_name = String::from("Megabux Supergaming");
        msg = Message{ player_id: Some(player_id), game_id: None, reply_channel: game_sender, msg: Request::NewGame(game_name) };
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
        let (mut game_sender, mut game_receiver) = channel();
        let player_name = String::from("Lizard");
        let mut msg = Message { player_id: None, game_id: None, reply_channel: game_sender, msg: Request::NewPlayer(player_name)};
        assert!(game_input_channel.send(msg).await.is_ok());

        let gm_id = match game_receiver.await {
            Ok(Outcome::NewPlayer(player_obj)) => player_obj.player_id, 
            _ => panic!("Expected Outcome::NewPlayer")
        };

        debug!("Creating new game.");

        (game_sender, game_receiver) = channel();
        let game_name = String::from("Thare She Blows: The Worst Campaign in Human History");
        msg = Message{ player_id: Some(gm_id), game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::NewGame(game_name) };

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
        let (gm_id, game_id) = add_new_game(&game_input_channel).await;

        let msg = Message { player_id: Some(gm_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::Delete };
        
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

        let (gm_id, game_id) = add_new_game(&game_input_channel).await;
        let player_name = String::from("Lizard");

        let mut msg = Message {player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::NewPlayer(player_name)};
        let mut send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());
        let (melf_id, mut melf_notifications) = match game_receiver.await.unwrap()
        {
            Outcome::NewPlayer(player_struct) => (player_struct.player_id, player_struct.player_1_receiver),
            _ => {panic!("These match arms should not have been invoked.")}
        };

        (game_sender, game_receiver) = channel::<Outcome>();
        let player_name = String::from("Wizard");
        msg = Message {player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::NewPlayer(player_name)};
        send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());
        let (mork_id, mut mork_notifications) = match game_receiver.await.unwrap()
        {
            Outcome::NewPlayer(player_struct) => (player_struct.player_id, player_struct.player_1_receiver),
            _ => {panic!("These match arms should not have been invoked.")}
        };

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message {player_id: Some(melf_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame};
        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message {player_id: Some(mork_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame};
        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        (game_sender, game_receiver) = channel::<Outcome>();

        msg = Message {player_id: Some(gm_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::Delete};
        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        
        loop
        {
            let change_option = mork_notifications.try_recv();
            match change_option
            {
                Ok(change_notice) => 
                {
                    match change_notice.as_ref()
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
                    match change_notice.as_ref()
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
    }


    #[tokio::test]
    pub async fn deleting_a_game_with_an_unknown_id_will_generate_unauthorized_action()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<Outcome>();

        let (gm_id, _game_id) = add_new_game(&game_input_channel).await;

        let msg = Message { player_id: Some(gm_id), game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::Delete };
        
        let send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());

        let response = game_receiver.await;
        assert!(response.is_ok());

        match response.unwrap()
        {
            Outcome::Destroyed => {panic!("The game deleted somehow - received a Destroyed message instead of an error.");},
            Outcome::Error(err) => 
            {
                assert!(err.kind == ErrorKind::UnauthorizedAction);
            }
            _ => {panic!("Received ResponseMessage that should not have been generated by request.");}
        }
    }

    #[tokio::test]
    pub async fn adding_a_new_character_to_a_valid_game_roster_generates_character_added_message()
    {
        let game_input_channel = init();
        let (mut game_sender, mut game_receiver) = channel::<Outcome>();

        let (_, game_id) = add_new_game(&game_input_channel).await;
        let player_name = String::from("Lizard");

        let mut msg = Message {player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::NewPlayer(player_name)};
        let send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());

        let player_id = match game_receiver.await {
            Ok(Outcome::NewPlayer(player_obj)) => {player_obj.player_id}
            _ => {panic!("Unexpected response from adding new player.")}
        };

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message {player_id: Some(player_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame};
        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());


        let character = create_character();
        (game_sender, game_receiver) = channel::<Outcome>();

        msg = Message { player_id: Some(player_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::AddCharacter(character) };
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
    pub async fn adding_a_character_to_a_non_extant_game_will_generate_unauthorized_action()
    {
        let game_input_channel = init();
        let (mut game_sender, mut game_receiver) = channel::<Outcome>();

        let (_, game_id) = add_new_game(&game_input_channel).await;
        let player_name = String::from("Lizard");

        let mut msg = Message {player_id: None, game_id: Some(game_id), reply_channel: game_sender, msg: Request::NewPlayer(player_name)};
        let send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());

        let player_id = match game_receiver.await {
            Ok(Outcome::NewPlayer(player_obj)) => {player_obj.player_id}
            _ => {panic!("Unexpected response from adding new player.")}
        };

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message {player_id: Some(player_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame};
        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        let character = create_character();
        (game_sender, game_receiver) = channel::<Outcome>();

        msg = Message { player_id: Some(player_id), game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::AddCharacter(character) };
        let send_state = game_input_channel.send(msg).await;

        assert!(send_state.is_ok());

        let response = game_receiver.await;
        assert!(response.is_ok());

        match response.unwrap()
        {
            Outcome::Error(err) => 
            { 
                assert!(err.kind == ErrorKind::UnauthorizedAction);
            },
            Outcome::CharacterAdded(_) => {panic!("This add should have failed - should have received Error rather than CharacterAdded.")},
            _ => {panic!("Another message was triggered, but add new character should result only in error or character added.")}
        }
    }

    #[tokio::test]
    pub async fn a_player_may_add_multiple_characters_and_review_them_with_get_pc_cast()
    {
        let game_channel = init();

        let (mut game_sender, mut game_receiver) = channel::<Outcome>();
        
        let mut msg = Message { player_id: None, game_id: None, reply_channel: game_sender, msg: Request::NewPlayer(String::from("Barnacles McGee"))};
        assert!(game_channel.send(msg).await.is_ok());

        let player_id = match game_receiver.await {Ok(Outcome::NewPlayer(player_id)) => player_id.player_id, _ => panic!("Registration failed.")};

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(player_id.clone()), game_id: None, reply_channel: game_sender, msg: Request::NewGame(String::from("Underwater Otter"))};
        assert!(game_channel.send(msg).await.is_ok());

        let game_id = match game_receiver.await { Ok(Outcome::Created(game_id)) => game_id, _ => panic!("Failed to create game.")};
        
        (game_sender, game_receiver) = channel::<Outcome>();
        let char = Character::new_pc(Metatypes::Dwarf, String::from("Thring Fringlonger"));
        msg = Message { player_id: Some(player_id.clone()), game_id: Some(game_id.clone()), reply_channel: game_sender, msg: Request::AddCharacter(char)};
        assert!(game_channel.send(msg).await.is_ok());

        let thring_id = match game_receiver.await { Ok(Outcome::CharacterAdded((_, char_id))) => char_id, _ => panic!("Character add failed.")};

        (game_sender, game_receiver) = channel::<Outcome>();
        let char = Character::new_pc(Metatypes::Dwarf, String::from("Hoola Hupz"));
        msg = Message { player_id: Some(player_id.clone()), game_id: Some(game_id.clone()), reply_channel: game_sender, msg: Request::AddCharacter(char)};
        assert!(game_channel.send(msg).await.is_ok());

        let hupz_id = match game_receiver.await { Ok(Outcome::CharacterAdded((_, char_id))) => char_id, _ => panic!("Character add failed.")};

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(player_id.clone()), game_id: Some(game_id.clone()), reply_channel: game_sender, msg: Request::GetPcCast};
        assert!(game_channel.send(msg).await.is_ok());

        let pcs = match game_receiver.await { Ok(Outcome::CastList(chars)) => chars, _ => panic!("Retrieve player characters failed")};

        assert_eq!(2, pcs.len());
        assert!(pcs.iter().all(|cs| cs.id == thring_id || cs.id == hupz_id))
    }


    #[tokio::test]
    pub async fn a_player_may_not_view_another_players_characters_with_get_pc_cast()
    {
        let game_channel = init();

        let (mut game_sender, mut game_receiver) = channel::<Outcome>();
        
        let mut msg = Message { player_id: None, game_id: None, reply_channel: game_sender, msg: Request::NewPlayer(String::from("Barnacles McGee"))};
        assert!(game_channel.send(msg).await.is_ok());

        let gm_id = match game_receiver.await {Ok(Outcome::NewPlayer(player_id)) => player_id.player_id, _ => panic!("Registration failed.")};

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(gm_id.clone()), game_id: None, reply_channel: game_sender, msg: Request::NewGame(String::from("Underwater Otter"))};
        assert!(game_channel.send(msg).await.is_ok());

        let game_id = match game_receiver.await { Ok(Outcome::Created(game_id)) => game_id, _ => panic!("Failed to create game.")};

        (game_sender, game_receiver) = channel::<Outcome>();
        let mut msg = Message { player_id: None, game_id: None, reply_channel: game_sender, msg: Request::NewPlayer(String::from("Barnacles McGee"))};
        assert!(game_channel.send(msg).await.is_ok());

        let player_1_id = match game_receiver.await {Ok(Outcome::NewPlayer(player_id)) => player_id.player_id, _ => panic!("Registration failed.")};

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(player_1_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame};
        assert!(game_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        (game_sender, game_receiver) = channel::<Outcome>();
        let char = Character::new_pc(Metatypes::Dwarf, String::from("Thring Fringlonger"));
        msg = Message { player_id: Some(player_1_id.clone()), game_id: Some(game_id.clone()), reply_channel: game_sender, msg: Request::AddCharacter(char)};
        assert!(game_channel.send(msg).await.is_ok());

        let thring_id = match game_receiver.await { Ok(Outcome::CharacterAdded((_, char_id))) => char_id, _ => panic!("Character add failed.")};

        (game_sender, game_receiver) = channel::<Outcome>();
        let mut msg = Message { player_id: None, game_id: None, reply_channel: game_sender, msg: Request::NewPlayer(String::from("Johnny B. Goode"))};
        assert!(game_channel.send(msg).await.is_ok());

        let player_2_id = match game_receiver.await {Ok(Outcome::NewPlayer(player_id)) => player_id.player_id, _ => panic!("Registration failed.")};

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(player_2_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame};
        assert!(game_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        (game_sender, game_receiver) = channel::<Outcome>();
        let char = Character::new_pc(Metatypes::Dwarf, String::from("Thring Fringlonger"));
        msg = Message { player_id: Some(player_2_id.clone()), game_id: Some(game_id.clone()), reply_channel: game_sender, msg: Request::AddCharacter(char)};
        assert!(game_channel.send(msg).await.is_ok());

        match game_receiver.await { Ok(Outcome::CharacterAdded((_, _))) => {}, _ => panic!("Character add failed.")};

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(player_1_id.clone()), game_id: Some(game_id.clone()), reply_channel: game_sender, msg: Request::GetPcCast};
        assert!(game_channel.send(msg).await.is_ok());

        let barnacles_chars = match game_receiver.await {Ok(Outcome::CastList(chars)) => chars, _ => panic!("PC retrieval failed")};

        assert_eq!(1, barnacles_chars.len());
        assert_eq!(thring_id, barnacles_chars.get(0).unwrap().id);
    }

    #[tokio::test]
    pub async fn a_gm_will_always_receive_the_entire_cast_list_with_get_pc_cast()
    {
        let game_channel = init();

        let (mut game_sender, mut game_receiver) = channel::<Outcome>();
        
        let mut msg = Message { player_id: None, game_id: None, reply_channel: game_sender, msg: Request::NewPlayer(String::from("Barnacles McGee"))};
        assert!(game_channel.send(msg).await.is_ok());

        let gm_id = match game_receiver.await {Ok(Outcome::NewPlayer(player_id)) => player_id.player_id, _ => panic!("Registration failed.")};

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(gm_id.clone()), game_id: None, reply_channel: game_sender, msg: Request::NewGame(String::from("Underwater Otter"))};
        assert!(game_channel.send(msg).await.is_ok());

        let game_id = match game_receiver.await { Ok(Outcome::Created(game_id)) => game_id, _ => panic!("Failed to create game.")};

        (game_sender, game_receiver) = channel::<Outcome>();
        let mut msg = Message { player_id: None, game_id: None, reply_channel: game_sender, msg: Request::NewPlayer(String::from("Barnacles McGee"))};
        assert!(game_channel.send(msg).await.is_ok());

        let player_1_id = match game_receiver.await {Ok(Outcome::NewPlayer(player_id)) => player_id.player_id, _ => panic!("Registration failed.")};

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(player_1_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame};
        assert!(game_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        (game_sender, game_receiver) = channel::<Outcome>();
        let char = Character::new_pc(Metatypes::Dwarf, String::from("Thring Fringlonger"));
        msg = Message { player_id: Some(player_1_id.clone()), game_id: Some(game_id.clone()), reply_channel: game_sender, msg: Request::AddCharacter(char)};
        assert!(game_channel.send(msg).await.is_ok());

        let thring_id = match game_receiver.await { Ok(Outcome::CharacterAdded((_, char_id))) => char_id, _ => panic!("Character add failed.")};

        (game_sender, game_receiver) = channel::<Outcome>();
        let mut msg = Message { player_id: None, game_id: None, reply_channel: game_sender, msg: Request::NewPlayer(String::from("Johnny B. Goode"))};
        assert!(game_channel.send(msg).await.is_ok());

        let player_2_id = match game_receiver.await {Ok(Outcome::NewPlayer(player_id)) => player_id.player_id, _ => panic!("Registration failed.")};

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(player_2_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::JoinGame};
        assert!(game_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        (game_sender, game_receiver) = channel::<Outcome>();
        let char = Character::new_pc(Metatypes::Dwarf, String::from("Thring Fringlonger"));
        msg = Message { player_id: Some(player_2_id.clone()), game_id: Some(game_id.clone()), reply_channel: game_sender, msg: Request::AddCharacter(char)};
        assert!(game_channel.send(msg).await.is_ok());

        let hupz_id = match game_receiver.await { Ok(Outcome::CharacterAdded((_, char_id))) => char_id, _ => panic!("Character add failed.")};

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(gm_id.clone()), game_id: Some(game_id.clone()), reply_channel: game_sender, msg: Request::GetPcCast};
        assert!(game_channel.send(msg).await.is_ok());

        let all_chars = match game_receiver.await {Ok(Outcome::CastList(chars)) => chars, _ => panic!("PC retrieval failed")};

        assert_eq!(2, all_chars.len());
        assert!(all_chars.iter().all(|c| c.id == thring_id || c.id == hupz_id));
    }

    #[tokio::test]
    pub async fn starting_combat_with_registered_characters_will_generate_combat_started_message()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<Outcome>();

        let (gm_id, game_id) = add_new_game(&game_input_channel).await;

        let (_, character1) = create_and_add_char(&game_input_channel, game_id).await;
        let (_, character2) = create_and_add_char(&game_input_channel, game_id).await;
        let (_, character3) = create_and_add_char(&game_input_channel, game_id).await;
        let (_, character4) = create_and_add_char(&game_input_channel, game_id).await;

        let combatants = vec![character1, character2, character3, character4];

        let msg = Message { player_id: Some(gm_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(combatants) };

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

        let (gm_id, game_id) = add_new_game(&game_input_channel).await;

        let (_, _character1) = create_and_add_char(&game_input_channel, game_id).await;
        let (_, _character2) = create_and_add_char(&game_input_channel, game_id).await;
        let (_, _character3) = create_and_add_char(&game_input_channel, game_id).await;
        let (_, _character4) = create_and_add_char(&game_input_channel, game_id).await;

        let combatants = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];

        let msg = Message { player_id: Some(gm_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(combatants) };

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
        let (game_sender, game_receiver) = channel::<Outcome>();

        let (gm_id, game_id) = add_new_game(&game_input_channel).await;

        let msg = Message { player_id: Some(gm_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(Vec::<Uuid>::new()) };

        assert!(game_input_channel.send(msg).await.is_ok());
        
        // It is entirely acceptable to start a combat with no combatants.  Individual combatants can be added later,
        // or another batch of combatants can be added later.
        match game_receiver.await {
            Ok(Outcome::CombatStarted) => {},
            _ => panic!("Expected CombatStarted message.")
        }
    }

    #[tokio::test]
    pub async fn starting_combat_with_an_unregistered_game_id_will_generate_no_matching_game()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<Outcome>();

        let (gm_id, game_id) = add_new_game(&game_input_channel).await;

        let (_, character1) = create_and_add_char(&game_input_channel, game_id).await;
        let (_, character2) = create_and_add_char(&game_input_channel, game_id).await;
        let (_, character3) = create_and_add_char(&game_input_channel, game_id).await;
        let (_, character4) = create_and_add_char(&game_input_channel, game_id).await;

        let combatants = vec![character1, character2, character3, character4];

        let msg = Message { player_id: Some(gm_id), game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::StartCombat(combatants) };

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
                            ErrorKind::UnauthorizedAction => {}
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

        let (gm_id, game_id) = add_new_game(&game_input_channel).await;

        let (_, character1) = create_and_add_char(&game_input_channel, game_id).await;
        let (_, character2) = create_and_add_char(&game_input_channel, game_id).await;
        let (_, character3) = create_and_add_char(&game_input_channel, game_id).await;
        let (_, character4) = create_and_add_char(&game_input_channel, game_id).await;
        let combatants = vec![character1, character2, character3, character4];

        let msg = Message { player_id: Some(gm_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(combatants) };

        let _response = game_input_channel.send(msg).await;

        let (game_sender, game_receiver) = channel::<Outcome>();

        let msg = Message { player_id: Some(gm_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase };

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

        let (gm_id, game_id) = add_new_game(&game_input_channel).await;

        let _character1 = create_and_add_char(&game_input_channel, game_id).await;
        let _character2 = create_and_add_char(&game_input_channel, game_id).await;
        let _character3 = create_and_add_char(&game_input_channel, game_id).await;

        let msg = Message { player_id: Some(gm_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(Vec::<Uuid>::new()) };

        let _response = game_input_channel.send(msg).await;

        let (game_sender, game_receiver) = channel::<Outcome>();

        let msg = Message { player_id: Some(gm_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase };

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

        let (gm_id, game_id) = add_new_game(&game_input_channel).await;

        let (player1, character1) = create_and_add_char(&game_input_channel, game_id).await;
        let (_, character2) = create_and_add_char(&game_input_channel, game_id).await;

        let msg = Message { player_id: Some(gm_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(vec![character1, character2]) };

        assert!(game_input_channel.send(msg).await.is_ok());

        let (game_sender, _game_receiver) = channel::<Outcome>();
        let msg = Message { player_id: Some(gm_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase };
        assert!(game_input_channel.send(msg).await.is_ok());

        let (game_sender, game_receiver) = channel::<Outcome>();
        let roll: Roll = Roll{ character_id: character1, roll: 13 };
        let msg = Message { player_id: Some(player1), game_id: Some(game_id), reply_channel: game_sender, msg: Request::AddInitiativeRoll(roll) };
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

        let (gm_id, game_id) = add_new_game(&game_input_channel).await;

        let (player1, character1) = create_and_add_char(&game_input_channel, game_id).await;
        let (player2, character2) = create_and_add_char(&game_input_channel, game_id).await;
        let (player3, character3) = create_and_add_char(&game_input_channel, game_id).await;
        let (player4, character4) = create_and_add_char(&game_input_channel, game_id).await;
        let players = vec![player1, player2, player3, player4];
        let combatants = vec![character1, character2, character3, character4];

        let msg = Message { player_id: Some(gm_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(combatants.clone()) };

        assert!(game_input_channel.send(msg).await.is_ok());

        let (game_sender, _game_receiver) = channel::<Outcome>();
        let msg = Message { player_id: Some(gm_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase };
        assert!(game_input_channel.send(msg).await.is_ok());

        for i in 0..4
        {
            let (game_sender, game_receiver) = channel::<Outcome>();
            let player_id = players.get(i).unwrap();
            let character_id = combatants.get(i).unwrap();
            let roll: Roll = Roll{character_id: *character_id, roll: 13 };
            let msg = Message { player_id: Some(*player_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::AddInitiativeRoll(roll) };
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

    async fn construct_combat_ready_game() -> (Sender<Message>, PlayerId, GameId, HashMap<PlayerId, CharacterId>)
    {
        debug!("Started construct_combat_ready_game()");
        let game_input_channel = init();
        let (game_sender, _game_receiver) = channel::<Outcome>();

        let (gm, game_id) = add_new_game(&game_input_channel).await;

        debug!("GM {} has created game {}", gm, game_id);

        let (player1, character1) = create_and_add_char(&game_input_channel, game_id).await;
        let (player2, character2) = create_and_add_char(&game_input_channel, game_id).await;
        let (player3, character3) = create_and_add_char(&game_input_channel, game_id).await;
        let (player4, character4) = create_and_add_char(&game_input_channel, game_id).await;
        let combatants = vec![character1, character2, character3, character4];

        let msg = Message { player_id: Some(gm), game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(combatants.clone()) };

        assert!(game_input_channel.send(msg).await.is_ok());

        let (game_sender, _game_receiver) = channel::<Outcome>();
        let msg = Message { player_id: Some(gm), game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase };
        assert!(game_input_channel.send(msg).await.is_ok());

        let mut player_character_map = HashMap::<Uuid, Uuid>::new();
        player_character_map.insert(player1, character1);
        player_character_map.insert(player2, character2);
        player_character_map.insert(player3, character3);
        player_character_map.insert(player4, character4);

        return (game_input_channel, gm, game_id, player_character_map);
    }

    #[tokio::test]
    pub async fn sending_start_combat_round_before_all_combatants_have_sent_initiatives_generates_invalid_state_action()
    {
        let (game_input_channel, gm_id, game_id, player_char_map) = construct_combat_ready_game().await;

        let players = player_char_map.keys().collect::<Vec<&PlayerId>>();

        let (game_sender, _game_receiver) = channel::<Outcome>();
        let roll = Roll{ character_id: *player_char_map.get(players.get(0).unwrap()).unwrap(), roll: 23 };
        let msg = Message{player_id: Some(**players.get(0).unwrap()), game_id: Some(game_id), reply_channel: game_sender, msg: Request::AddInitiativeRoll(roll)};
        assert!(game_input_channel.send(msg).await.is_ok());
        
        let (game_sender, game_receiver) = channel::<Outcome>();
        let msg = Message { player_id: Some(gm_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombatRound };

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

        let (mut game_sender, mut game_receiver) = channel();
        let player_name = String::from("Lizard");

        let mut msg = Message { player_id: None, game_id: None, reply_channel: game_sender, msg: Request::NewPlayer(player_name)};
        assert!(game_input_channel.send(msg).await.is_ok());

        let gm_id = match game_receiver.await {
            Ok(Outcome::NewPlayer(player_obj)) => player_obj.player_id,
            _ => panic!("Expected NewPlayer message.")
        };

        (game_sender, game_receiver) = channel();
        let game_name = String::from("Pseudofed - The Illusory Banquet");
        msg = Message { player_id: Some(gm_id), game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::NewGame(game_name) };

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
        let msg = Message { player_id: Some(gm_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombatRound };

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
    pub async fn sending_start_combat_round_after_declaring_combat_generates_invalid_state_action()
    {
        let (game_input_channel, gm_id, game_id, _combatants) = construct_combat_ready_game().await;

        let (game_sender, game_receiver) = channel::<Outcome>();
        let msg = Message { player_id: Some(gm_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombatRound };

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

        let player_name = String::from("Lizard");
        let mut msg = Message {player_id: None, game_id: None, reply_channel: game_sender, msg: Request::NewPlayer(player_name)};
        assert!(game_input_channel.send(msg).await.is_ok());

        let player_id = match game_receiver.await
        {
            Ok(Outcome::NewPlayer(player_ob)) => {player_ob.player_id}
            _ => panic!("Should have received NewPlayer Outcome.")
        };

        (game_sender, game_receiver) = channel::<Outcome>();
        let game_name = String::from("Pinkerton's Detective Agency & Boulangerie Downstairs");
        msg = Message { player_id: Some(player_id), game_id: Some(Uuid::new_v4()), reply_channel: game_sender, msg: Request::NewGame(game_name) };
        

        assert!(game_input_channel.send(msg).await.is_ok());
        let game_id = match game_receiver.await {
            Ok(Outcome::Created(generated_id)) => generated_id, 
            _ => panic!("Expected Outcome::Created.  Was disappointed.")
        };

        let (player1, character1) = create_and_add_char(&game_input_channel, game_id).await;
        let (player2, character2) = create_and_add_char(&game_input_channel, game_id).await;

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(player_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase };
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
        msg = Message { player_id: Some(player_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombat(vec![character1, character2]) };
        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        (game_sender, game_receiver) = channel::<Outcome>();

        msg = Message { player_id: Some(player_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase };
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
        msg = Message{player_id: Some(player_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase};
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
        msg = Message { player_id: Some(player1), game_id: Some(game_id), reply_channel: game_sender, msg: Request::AddInitiativeRoll(Roll { character_id: character1, roll: 13 })};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(player2), game_id: Some(game_id), reply_channel: game_sender, msg: Request::AddInitiativeRoll(Roll { character_id: character2, roll: 23 })};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(player_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::StartCombatRound};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(player2), game_id: Some(game_id), reply_channel: game_sender, msg: Request::TakeAction(Action { character_id: character2, action: ActionType::Complex })};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(player_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::AdvanceTurn};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(player1), game_id: Some(game_id), reply_channel: game_sender, msg: Request::TakeAction(Action { character_id: character1, action: ActionType::Complex })};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { player_id: Some(player_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::AdvanceTurn};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message{player_id: Some(player_id), game_id: Some(game_id), reply_channel: game_sender, msg: Request::BeginInitiativePhase};
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
        let (sender, gm, game_id, player_char_map) = construct_combat_ready_game().await;

        let players = player_char_map.keys().collect::<Vec<&PlayerId>>();

        let (mut game_owned_sender, mut our_receiver) = channel::<Outcome>();
        let mut msg = Message{ player_id: Some(**players.get(0).unwrap()), game_id: Some(game_id), reply_channel: game_owned_sender, 
            msg: Request::AddInitiativeRoll(Roll{ character_id: *player_char_map.get(players.get(0).unwrap()).unwrap(), roll: 13 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: Some(**players.get(1).unwrap()), game_id: Some(game_id), reply_channel: game_owned_sender, msg: 
            Request::AddInitiativeRoll(Roll{ character_id: *player_char_map.get(players.get(1).unwrap()).unwrap(), roll: 23 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: Some(**players.get(2).unwrap()), game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *player_char_map.get(players.get(2).unwrap()).unwrap(), roll: 9 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: Some(**players.get(3).unwrap()), game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *player_char_map.get(players.get(3).unwrap()).unwrap(), roll: 16 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());

        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: Some(gm), game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::StartCombatRound};
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: Some(**players.get(1).unwrap()), game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::TakeAction
            (Action{character_id: *player_char_map.get(players.get(1).unwrap()).unwrap(), action: ActionType::Complex})};
        
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
        let (sender, gm, game_id, player_char_map) = construct_combat_ready_game().await;

        let players = player_char_map.keys().collect::<Vec<&PlayerId>>();

        let (mut game_owned_sender, mut our_receiver) = channel::<Outcome>();
        let mut msg = Message{ player_id: Some(**players.get(0).unwrap()), game_id: Some(game_id), reply_channel: game_owned_sender, 
            msg: Request::AddInitiativeRoll(Roll{ character_id: *player_char_map.get(players.get(0).unwrap()).unwrap(), roll: 13 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: Some(**players.get(1).unwrap()), game_id: Some(game_id), reply_channel: game_owned_sender, 
            msg: Request::AddInitiativeRoll(Roll{ character_id: *player_char_map.get(players.get(1).unwrap()).unwrap(), roll: 23 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: Some(**players.get(2).unwrap()), game_id: Some(game_id), reply_channel: game_owned_sender, 
            msg: Request::AddInitiativeRoll(Roll{ character_id: *player_char_map.get(players.get(2).unwrap()).unwrap(), roll: 9 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: Some(**players.get(3).unwrap()), game_id: Some(game_id), reply_channel: game_owned_sender, 
            msg: Request::AddInitiativeRoll(Roll{ character_id: *player_char_map.get(players.get(3).unwrap()).unwrap(), roll: 16 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());

        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: Some(gm), game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::StartCombatRound};
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());

        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: Some(**players.get(2).unwrap()), game_id: Some(game_id), reply_channel: game_owned_sender, 
            msg: Request::TakeAction(Action{ character_id: *player_char_map.get(players.get(2).unwrap()).unwrap(), action: ActionType::Free })};
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
        let (sender, gm, game_id, player_char_map) = construct_combat_ready_game().await;

        let players = player_char_map.keys().collect::<Vec<&PlayerId>>();
        let player1 = players.get(0).unwrap();
        let player2 = players.get(1).unwrap();
        let player3 = players.get(2).unwrap();
        let player4 = players.get(3).unwrap();
        let character1 = player_char_map.get(player1).unwrap();
        let character2 = player_char_map.get(player2).unwrap();
        let character3 = player_char_map.get(player3).unwrap();
        let character4 = player_char_map.get(player4).unwrap();

        let (mut game_owned_sender, mut our_receiver) = channel::<Outcome>();
        let mut msg = Message{ player_id: Some(**player1), game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *character1, roll: 13 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: Some(**player2), game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *character2, roll: 23 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: Some(**player3), game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *character3, roll: 9 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: Some(**player4), game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::AddInitiativeRoll
            (Roll{ character_id: *character4, roll: 16 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());

        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: Some(gm), game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::StartCombatRound};
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());

        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ player_id: Some(**player3), game_id: Some(game_id), reply_channel: game_owned_sender, msg: Request::TakeAction
            (Action{ character_id: *character3, action: ActionType::Complex })};
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