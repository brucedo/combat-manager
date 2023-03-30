use std::{collections::{HashMap, HashSet}, sync::Arc};

use log::{debug, error};
use tokio::sync::mpsc::{Receiver, Sender as MpscSender, channel};
use tokio::sync::oneshot::Sender as OneShotSender;
use uuid::Uuid;

use crate::{tracker::{game::{Game, ActionType}, character::Character}, gamerunner::registry::GameRegistry};

pub mod registry;

type NotifyList = Vec<Uuid>;

pub async fn game_runner(mut message_queue: Receiver<Message>)
{
    debug!("Game runner redux started.");

    let mut directory = GameRegistry::new();

    while let Some(message) = message_queue.recv().await
    {
        let game_id = message.game_id;
        let (channel, game_id, msg) = (message.reply_channel, message.game_id, message.msg);
        let (response, to_notify) = dispatch_message(&mut directory, game_id, msg);

        notify_players(&mut directory, to_notify);

        if channel.send(response).is_err()
        {
            error!("The return channel has dropped.");
        }
    }
}

fn dispatch_message(registry: &mut GameRegistry, game_id: GameId, msg: Event) -> (Outcome, Option<Vec<Uuid>>)
{
    
    match msg
    {
        Event::NewPlayer => {
            debug!("Request is to register as a player.");
            (register_player(registry), None)
        },
        Event::Enumerate => {
            debug!("Request is for a list of running games.");
            (enumerate(registry), None)
        }
        Event::New => {
            debug!("Request is for new game.");
            (new_game(registry), None)
        },
        Event::Delete => {
            debug!("Request is to remove game.");
            end_game(game_id, registry)
        },
        Event::JoinGame(player_id) => {
            debug!("Request is to let a player join a game.");
            join_game(player_id, game_id, registry)
        
        },
        Event::AddCharacter(character) => {
            debug!("Request is to add a new character.");
            find_game_and_act(registry, game_id, | game | {add_character(character, game)})
        },
        Event::GetFullCast => {
            debug!("Request is to get the full cast list.");
            find_game_and_act(registry, game_id, get_full_cast)
        },
        Event::GetNpcCast => {
            debug!("Request is to get the NPC cast list.");
            find_game_and_act(registry, game_id, get_npcs)
        },
        Event::GetPcCast => {
            debug!("Reqeust is to get the PC cast list.");
            find_game_and_act(registry, game_id, get_pcs)
        }
        Event::GetCharacter(id) => {
            debug!("Request is to get a character by id.");
            find_game_and_act(registry, game_id, |game| {get_char(id, game)})
        }
        Event::StartCombat(combatants) => {
            debug!("Request is to start the combat phase.");                
            find_game_and_act(registry, game_id, | game | {start_combat(combatants.to_owned(), game)})
        },
        Event::AddInitiativeRoll(roll) => {
            debug!("Request is to add an initiative roll.");
            find_game_and_act(registry, game_id, | game | { add_init_roll(roll, game)})
        },
        Event::BeginInitiativePhase => {
            debug!("Request is to begin the initiative phase.");
            find_game_and_act(registry, game_id, try_initiative_phase)
        },
        Event::StartCombatRound => {
            debug!("Request is to begin a combat round.");
            find_game_and_act( registry, game_id, try_begin_combat)
        },
        Event::TakeAction(action) =>
        {
            debug!("Request is for some character to perform some action.");
            find_game_and_act( registry, game_id, | game | {take_action(game, action)})
        }
        Event::AdvanceTurn => {
            debug!("Request is to advance to the next event in the pass.");
            find_game_and_act( registry, game_id, try_advance_turn)
        }
        Event::WhoGoesThisTurn => {
            debug!("Request is to see who is going this turn.");
            find_game_and_act(registry, game_id, list_current_turn_events)
        }
        Event::WhatHasYetToHappenThisTurn => {
            debug!("Request is to see who has yet to go.");
            find_game_and_act(registry, game_id, list_unresolved_events)
        }
        Event::WhatHappensNextTurn => {
            debug!("Request is to see what happens next turn.");
            find_game_and_act(registry, game_id, list_next_turn_events)
        }
        Event::AllEventsThisPass => {
            debug!("Request is for a full accounting of all events on this pass.");
            find_game_and_act(registry, game_id, list_all_events_by_id_this_pass)
        }
        Event::NextInitiative => {
            debug!("Request is to get the next initiative number.");
            find_game_and_act(registry, game_id, next_initiative)
        }
        Event::CurrentInitiative => {
            debug!("Request is to get the current initiative number.");
            find_game_and_act(registry, game_id, current_initiative)
        }
        Event::AllRemainingInitiatives => {
            debug!("Request is to get any initiatives that have not been fully resolved.");
            find_game_and_act(registry, game_id, remaining_initiatives_are)
        }
        _ => {todo!()}
    }
}

fn notify_players(registry: &mut GameRegistry, notification_list: Option<NotifyList>)
{
    match notification_list 
    {
        Some(list) => 
        {
            for player in list
            {
                if let Some(sender) = registry.get_player_sender(&player)
                {
                    sender.try_send(WhatChanged::GameEnded);
                }
            }
        }, 
        None => {}
    }
}

pub async fn game_runner_old(mut message_queue: Receiver<Message>)
{

    // set up whatever we're going to use to store running games.  For now, a simple HashMap will do.
    debug!("Game runner started.");
    let mut directory = GameRegistry::new();

    while let Some(message) = message_queue.recv().await
    {
        debug!("Received request.  processing...");
        let channel: OneShotSender<Outcome> = message.reply_channel;
        let game_id = message.game_id;
        let response = match message.msg {
            Event::NewPlayer => {
                debug!("Request is to register as a player.");
                register_player(&mut directory)
            }
            Event::Enumerate => {
                debug!("Request is for a list of running games.");
                enumerate(&mut directory)
            }
            Event::New => {
                debug!("Request is for new game.");
                new_game(&mut directory)
            },
            Event::Delete => {
                debug!("Request is to remove game.");
                end_game(game_id, &mut directory).0
            },
            Event::JoinGame(player_id) => {
                debug!("Request is to let a player join a game.");
                join_game(player_id, game_id, &mut directory).0
            
            },
            Event::AddCharacter(character) => {
                debug!("Request is to add a new character.");
                find_game_and_act(&mut directory, game_id, | game | {add_character(character, game)}).0
            },
            Event::GetFullCast => {
                debug!("Request is to get the full cast list.");
                find_game_and_act(&mut directory, game_id, get_full_cast).0
            },
            Event::GetNpcCast => {
                debug!("Request is to get the NPC cast list.");
                find_game_and_act(&mut directory, game_id, get_npcs).0
            },
            Event::GetPcCast => {
                debug!("Reqeust is to get the PC cast list.");
                find_game_and_act(&mut directory, game_id, get_pcs).0
            }
            Event::GetCharacter(id) => {
                debug!("Request is to get a character by id.");
                find_game_and_act(&mut directory, game_id, |game| {get_char(id, game)}).0
            }
            Event::StartCombat(combatants) => {
                debug!("Request is to start the combat phase.");                
                find_game_and_act(&mut directory, game_id, | game | {start_combat(combatants, game)}).0
            },
            Event::AddInitiativeRoll(roll) => {
                debug!("Request is to add an initiative roll.");
                find_game_and_act(&mut directory, game_id, | game | { add_init_roll(roll, game)}).0
            },
            Event::BeginInitiativePhase => {
                debug!("Request is to begin the initiative phase.");
                find_game_and_act(&mut directory, game_id, try_initiative_phase).0
            },
            Event::StartCombatRound => {
                debug!("Request is to begin a combat round.");
                find_game_and_act(&mut directory, game_id, try_begin_combat).0
            },
            Event::TakeAction(action) =>
            {
                debug!("Request is for some character to perform some action.");
                find_game_and_act(&mut directory, game_id, | game | {take_action(game, action)}).0
            }
            Event::AdvanceTurn => {
                debug!("Request is to advance to the next event in the pass.");
                find_game_and_act(&mut directory, game_id, try_advance_turn).0
            }
            Event::WhoGoesThisTurn => {
                debug!("Request is to see who is going this turn.");
                find_game_and_act(&mut directory, game_id, list_current_turn_events).0
            }
            Event::WhatHasYetToHappenThisTurn => {
                debug!("Request is to see who has yet to go.");
                find_game_and_act(&mut directory, game_id, list_unresolved_events).0
            }
            Event::WhatHappensNextTurn => {
                debug!("Request is to see what happens next turn.");
                find_game_and_act(&mut directory, game_id, list_next_turn_events).0
            }
            Event::AllEventsThisPass => {
                debug!("Request is for a full accounting of all events on this pass.");
                find_game_and_act(&mut directory, game_id, list_all_events_by_id_this_pass).0
            }
            Event::NextInitiative => {
                debug!("Request is to get the next initiative number.");
                find_game_and_act(&mut directory, game_id, next_initiative).0
            }
            Event::CurrentInitiative => {
                debug!("Request is to get the current initiative number.");
                find_game_and_act(&mut directory, game_id, current_initiative).0
            }
            Event::AllRemainingInitiatives => {
                debug!("Request is to get any initiatives that have not been fully resolved.");
                find_game_and_act(&mut directory, game_id, remaining_initiatives_are).0
            }
            _ => { todo!()}
        };

        if channel.send(response).is_err()
        {
            error!("The return channel has dropped.");
        }
    }
}

fn enumerate(running_games: &mut GameRegistry ) -> Outcome
{

    let games = running_games.enumerate_games();

    let mut enumeration = Vec::<(Uuid, String)>::with_capacity(games.len());
    
    for id in games
    {
        enumeration.push((id, String::from("")));
    }

    return Outcome::Summaries(enumeration);
}

fn new_game(running_games: &mut GameRegistry) -> Outcome
{
    let response: Outcome;

    let game_id = Uuid::new_v4();
    running_games.new_game(game_id, Game::new());
    response = Outcome::Created(game_id);

    return response;
}

fn end_game(game: Uuid, directory: &mut GameRegistry) -> (Outcome, Option<NotifyList>)
{
    let response: Outcome;
    let mut to_notify: Option<Vec<Uuid>> = None;
    let mut affected_players = Vec::new();
    if let Some(players) = directory.players_by_game(game)
    {
        for player in players
        {
            affected_players.push(*player);
        }
        to_notify = Some(affected_players);
    }

    match directory.delete_game(game)
    {
        Ok(_) => {response = Outcome::Destroyed},
        Err(_) => {response = Outcome::Error(
            Error{ message: String::from(format!("No game by ID {} exists.", game.clone())), kind: ErrorKind::NoMatchingGame })},
    }

    return (response, to_notify);
}

fn register_player(player_directory: &mut GameRegistry) -> Outcome
{
    let mut player_id = Uuid::new_v4();

    while player_directory.is_registered(player_id)
    {
        player_id = Uuid::new_v4();
    }
    let (player_sender, player_receiver) = channel(32);
    let player_info = NewPlayer{ player_id, player_receiver };   

    match player_directory.register_player(player_id, player_sender)
    {
        Ok(_) => {Outcome::NewPlayer(player_info)},
        Err(_) => {unreachable!("Duplicate ID encountered despite explicitly checking for duplicate ID before joining")}
    }

    // return Outcome::NewPlayer(player_info);
}

fn is_registered_player(player_id: Uuid, player_directory: &GameRegistry) -> bool
{
    player_directory.is_registered(player_id)
}

fn join_game(player_id: Uuid, game_id: Uuid, game_directory: &mut GameRegistry) -> (Outcome, Option<NotifyList>)
{
    let mut to_notify: Option<NotifyList> = None;
    match game_directory.join_game(player_id, game_id)
    {
        Ok(_) => 
        {
            // let potential_players = game_directory.players_by_game(game_id);
            let mut affected_players = NotifyList::new();
            if let Some(potential_players) = game_directory.players_by_game(game_id)
            {
                for player in potential_players
                {
                    affected_players.push(*player);
                }
                to_notify = Some(affected_players);
            }
            
            (Outcome::JoinedGame(GameState {}), to_notify)
        },
        Err(_) => 
        {
            (Outcome::Error(Error { message: String::from(format!("No matching game for id {}", game_id)), kind: ErrorKind::NoMatchingGame }), to_notify)
        },
    }
}

fn add_character(character: Character, game: &mut Game) -> Outcome
{
    let char_id = game.add_cast_member(character);
    return Outcome::CharacterAdded(char_id);
}

fn get_full_cast(game: &mut Game) -> Outcome
{
    Outcome::CastList(game.get_cast())
}

fn get_npcs(game: &mut Game) -> Outcome
{
    Outcome::CastList(game.get_npcs())
}

fn get_pcs(game: &mut Game) -> Outcome
{
    Outcome::CastList(game.get_pcs())
}

fn get_char(char_id:Uuid, game: &mut Game) -> Outcome
{
    Outcome::Found(game.get_cast_by_id(char_id))
}

fn start_combat(combatants: Vec<Uuid>, game: &mut Game) -> Outcome
{

    let response: Outcome;

    if let Err(result) = game.add_combatants(combatants)
    {
        match result.kind
        {
            crate::tracker::game::ErrorKind::UnknownCastId => {
                response = Outcome::Error
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
        response = Outcome::CombatStarted;
    }

    return response;

}

fn try_initiative_phase(game: &mut Game) -> Outcome
{
    let response: Outcome;

    match game.start_initiative_phase()
    {
        Ok(_) => {
            debug!("Non-error returned from game.start_initiative_phase()");
            response = Outcome::InitiativePhaseStarted;
        },
        Err(game_err) => {
            let runner_err: Error;
            match game_err.kind
            {
                crate::tracker::game::ErrorKind::InvalidStateAction => 
                {
                    runner_err = Error {kind: ErrorKind::InvalidStateAction, message: game_err.msg}
                },
                crate::tracker::game::ErrorKind::UnknownCastId => 
                {
                    runner_err = Error {kind: ErrorKind::NoSuchCharacter, message: game_err.msg}
                }
                crate::tracker::game::ErrorKind::UnresolvedCombatant => 
                {
                    runner_err = Error {kind: ErrorKind::UnresolvedCombatant, message: game_err.msg}
                },
                _ => {unreachable!()}
            }
            error!("Error returned from game.start_initiative_phase()");
            response = Outcome::Error(runner_err);
        },
    }

    return response;
}

// fn add_init_roll(character_id: Uuid, roll: i8, game: &mut Game) -> Outcome
fn add_init_roll(roll: Roll, game: &mut Game) -> Outcome
{
    let response: Outcome;

    if let Err(result) = game.accept_initiative_roll(roll.character_id, roll.roll)
    {
        match result.kind
        {
            crate::tracker::game::ErrorKind::InvalidStateAction => {
                response = Outcome::Error
                (
                    Error 
                    { 
                        message: String::from(format!("The game is not in the correct state to take initiative rolls.")), 
                        kind: ErrorKind::InvalidStateAction 
                    }
                );
            },
            crate::tracker::game::ErrorKind::UnknownCastId => {
                response = Outcome::Error
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
        response = Outcome::InitiativeRollAdded;
    }

    return response;

}

fn try_begin_combat(game: &mut Game) -> Outcome
{
    
    let response: Outcome;

    if let Err(err) = game.start_combat_rounds()
    {
        match err.kind
        {
            crate::tracker::game::ErrorKind::InvalidStateAction => {
                response = Outcome::Error(Error{ message: err.msg, kind: ErrorKind::InvalidStateAction })
            },
            _ => {unreachable!()}
        }
    }
    else 
    {
        response = Outcome::CombatRoundStarted;    
    }

    return response;
}

pub fn try_advance_turn(game: &mut Game) -> Outcome
{
    let response: Outcome;

    if let Err(err) = game.advance_round()
    {
        match err.kind
        {
            crate::tracker::game::ErrorKind::InvalidStateAction => 
            {
                response = Outcome::Error(Error{message: err.msg, kind: ErrorKind::InvalidStateAction});
            },
            crate::tracker::game::ErrorKind::UnresolvedCombatant => 
            {
                response = Outcome::Error(Error{message: err.msg, kind: ErrorKind::CannotAdvanceTurn})
            },
            crate::tracker::game::ErrorKind::EndOfInitiative =>
            {
                response = Outcome::Error(Error{message: err.msg, kind: ErrorKind::NoEventsLeft})
            }
            _ => {unreachable!("Should not receive any other error from stepping the initiative forward.")}
        }
    }
    else
    {
        // let up = match game.waiting_for(){ Some(filled) => filled, None => Vec::<Uuid>::new() };
        // let on_deck = match game.on_deck(){ Some(filled) => filled, None => Vec::<Uuid>::new() };

        response = Outcome::TurnAdvanced;
    }

    return response;
}

fn take_action(game: &mut Game, action: Action) -> Outcome
{
    match game.take_action(action.character_id, action.action)
    {
        Ok(_) => 
        {
            return Outcome::ActionTaken
        },
        Err(err) => 
        {
            match err.kind
            {
                crate::tracker::game::ErrorKind::InvalidStateAction => 
                    {return Outcome::Error(Error{message: err.msg, kind: ErrorKind::InvalidStateAction})},
                crate::tracker::game::ErrorKind::UnknownCastId => 
                    {return Outcome::Error(Error{message: err.msg, kind: ErrorKind::NoSuchCharacter})},
                crate::tracker::game::ErrorKind::EndOfInitiative => 
                    {return Outcome::Error(Error{message:err.msg, kind: ErrorKind::CannotAdvanceTurn})},
                crate::tracker::game::ErrorKind::NoAction => 
                    {return Outcome::Error(Error{message: err.msg, kind: ErrorKind::NoActionLeft})},
                crate::tracker::game::ErrorKind::UnresolvedCombatant => 
                    {return Outcome::Error(Error{message: err.msg, kind: ErrorKind::NotCharactersTurn})},
                _ => {unreachable!("Should not be called.")}
            }
        },
    }
}

fn list_current_turn_events(game: &mut Game) -> Outcome
{
    Outcome::MatchingEventsAre(game.currently_up())
}

fn list_unresolved_events(game: &mut Game) -> Outcome
{
    Outcome::MatchingEventsAre(game.waiting_for())
}

fn list_next_turn_events(game: &mut Game) -> Outcome
{
    Outcome::MatchingEventsAre(game.on_deck())
}

fn list_all_events_by_id_this_pass(game: &mut Game) -> Outcome
{
    Outcome::MatchingEventsById(game.collect_all_remaining_events())
}

fn next_initiative(game: &mut Game) -> Outcome
{
    Outcome::InitiativeIs(game.get_next_init())
}

fn current_initiative(game: &mut Game) -> Outcome
{
    Outcome::InitiativeIs(game.get_current_init())
}

fn remaining_initiatives_are(game: &mut Game) -> Outcome
{
    Outcome::InitiativesAre(game.get_all_remaining_initiatives())
}

fn find_game_and_act<F>(running_games: &mut GameRegistry, game_id: Uuid, action: F) -> (Outcome, Option<NotifyList>)
where
    F: FnOnce(&mut Game) -> Outcome
{
    let response: Outcome;
    let mut send_to: Option<Vec<Uuid>> = None;

    if let Some(player_id_list) = running_games.players_by_game(game_id)
    {
        let mut temp = Vec::<Uuid>::new();
        for player_id in player_id_list
        {
            temp.push(*player_id);
        }
        send_to = Some(temp);
    }

    match running_games.get_mut_game(game_id)
    {
        Some(mut game) => 
        {
            response = action(&mut game);
        },
        None => {response = game_not_found(game_id)},
    }

    return (response, send_to);
}

fn game_not_found(id: Uuid) -> Outcome
{
    Outcome::Error
    (
        Error 
        { 
            message: String::from(format!("The ID provided ({}) has no associated game.", id)), 
            kind: ErrorKind::NoMatchingGame 
        }
    )
}

pub struct Message
{
    pub game_id: Uuid,
    pub reply_channel: OneShotSender<Outcome>,
    pub msg: Event,
}

type PlayerId = Uuid;
type GameId = Uuid;
pub enum Event
{
    Enumerate,
    New,
    Delete,
    NewPlayer,
    JoinGame(PlayerId),
    AddCharacter(Character),
    GetFullCast,
    GetNpcCast,
    GetPcCast,
    GetCharacter(Uuid),
    StartCombat(Vec<Uuid>),
    AddInitiativeRoll(Roll),
    BeginInitiativePhase,
    QueryInitiativePhase,
    StartCombatRound,
    TakeAction(Action),
    AdvanceTurn,
    AdvancePass,
    EndCombat,
    QueryCurrentState,
    QueryMissingInitiatives,
    WhoGoesThisTurn,
    WhatHasYetToHappenThisTurn,
    WhatHappensNextTurn,
    AllEventsThisPass,
    CurrentInitiative,
    NextInitiative,
    AllRemainingInitiatives,
    QueryAllCombatants,
    BeginEndOfTurn,
}

pub enum Outcome
{
    NewPlayer(NewPlayer),
    Summaries(Vec<(Uuid, String)>),
    JoinedGame(GameState),
    Created(Uuid),
    CastList(Vec<Arc<Character>>),
    Found(Option<Arc<Character>>),
    Destroyed,
    Error(Error),
    CharacterAdded(Uuid),
    CombatStarted,
    InitiativePhaseStarted,
    InitiativeRollAdded,
    InitiativeStatus(InitiativeState),
    CombatRoundStarted,
    ActionTaken,
    TurnAdvanced,
    CombatEnded,
    CurrentStateIs,
    MissingInitiativesFor,
    MatchingEventsAre(Option<Vec<Uuid>>),
    MatchingEventsById(Option<HashMap<i8, Vec<Uuid>>>),
    InitiativeIs(Option<i8>),
    InitiativesAre(Option<Vec<i8>>),
    AllCombatantsAre,
}

pub struct Action
{
    pub character_id: Uuid,
    pub action: ActionType
}


pub struct InitiativeState
{
    pub waiting: bool,
    pub remaining: Vec<Uuid>
}

pub struct AddCharacter
{
    pub reply_channel: OneShotSender<Outcome>,
    pub game_id: Uuid,
    pub character: Character,
}

pub struct Roll
{
    pub character_id: Uuid,
    pub roll: i8,
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

pub struct PlayerDirectoryEntry
{
    pub player_id: Uuid,
    pub player_games: HashSet<Uuid>,
    pub player_characters: HashSet<Uuid>,
    pub player_sender: MpscSender<GameUpdates>
}

pub struct GameDirectoryEntry
{
    pub game: Game,
    pub players: Vec<Uuid>,
}

pub struct NewPlayer
{
    pub player_id: Uuid,
    pub player_receiver: Receiver<WhatChanged>
}

pub struct GameState
{

}

pub struct GameUpdates
{
    
}

pub enum WhatChanged
{
    NewPlayer(String),
    TurnAdvanced,
    PassAdvanced,
    RoundAdvanced,
    CombatStarted,
    UpNext,
    YourTurn,
    CombatEnded,
    GameEnded,
}

#[derive(PartialEq)]
pub enum ErrorKind
{
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
    

    use crate::gamerunner::Action;
    use crate::gamerunner::{Outcome, game_runner, Event};
    use crate::tracker::character::Character;
    use crate::tracker::character::Metatypes;
    use crate::tracker::game::ActionType;
    use crate::gamerunner::WhatChanged;

    use super::ErrorKind;
    use super::Message;
    use super::NewPlayer;
    use super::Roll;

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
        let msg = Message { game_id: Uuid::new_v4(), reply_channel: game_sender, msg: Event::New };

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
        
        let msg = Message {game_id, reply_channel: game_sender, msg: Event::NewPlayer};

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

        let msg = Message {game_id, reply_channel: game_sender, msg: Event::NewPlayer};

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
        let msg = Message {game_id, reply_channel: game_sender, msg: Event::JoinGame(player_state.player_id)};

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
        let game_input_channel = init();
        let game_id = add_new_game(&game_input_channel).await;

        let NewPlayer {player_id: player_1_id, player_receiver: mut player_1_channel} = player_join_game(&game_input_channel, game_id).await;
        let (mut game_sender, mut game_receiver) = channel();
        let mut msg = Message {game_id, reply_channel: game_sender, msg: Event::JoinGame(player_1_id)};

        assert!(game_input_channel.send(msg).await.is_ok() );
        assert!(game_receiver.await.is_ok());

        let player_state = player_join_game(&game_input_channel, game_id).await;
        (game_sender, game_receiver) = channel();
        msg = Message {game_id, reply_channel: game_sender, msg: Event::JoinGame(player_state.player_id)};

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

        let msg = Message { game_id, reply_channel: game_sender, msg: Event::AddCharacter(character) };
        let send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());

        let response = game_receiver.await;
        assert!(response.is_ok());

        match response
        {
            Ok(msg) => {
                match msg
                {
                    Outcome::CharacterAdded(id) => {return id;}
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

        let msg = Message{ game_id: Uuid::new_v4(), reply_channel: game_sender, msg: Event::Enumerate };
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

        let msg = Message{ game_id: Uuid::new_v4(), reply_channel: game_sender, msg: Event::New };
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

        let msg = Message{ game_id: Uuid::new_v4(), reply_channel: game_sender, msg: Event::Enumerate };
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
        let msg = Message{ game_id: Uuid::new_v4(), reply_channel: game_sender, msg: Event::New };

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

        let msg = Message { game_id: game_id, reply_channel: game_sender, msg: Event::Delete };
        
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

        let mut msg = Message {game_id, reply_channel: game_sender, msg: Event::NewPlayer};
        let mut send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());
        let (melf_id, mut melf_notifications) = match game_receiver.await.unwrap()
        {
            Outcome::NewPlayer(player_struct) => (player_struct.player_id, player_struct.player_receiver),
            _ => {panic!("These match arms should not have been invoked.")}
        };

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message {game_id, reply_channel: game_sender, msg: Event::NewPlayer};
        send_state = game_input_channel.send(msg).await;
        assert!(send_state.is_ok());
        let (mork_id, mut mork_notifications) = match game_receiver.await.unwrap()
        {
            Outcome::NewPlayer(player_struct) => (player_struct.player_id, player_struct.player_receiver),
            _ => {panic!("These match arms should not have been invoked.")}
        };

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message {game_id, reply_channel: game_sender, msg: Event::JoinGame(mork_id)};
        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message {game_id, reply_channel: game_sender, msg: Event::JoinGame(melf_id)};
        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        (game_sender, game_receiver) = channel::<Outcome>();

        msg = Message {game_id, reply_channel: game_sender, msg: Event::Delete};
        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        match mork_notifications.recv().await
        {
            Some(change_notice) =>  match change_notice {
                WhatChanged::GameEnded => {},
                _ => {panic!("Should have received game ended notification")}
            },
            None => { panic!("Should have produced a WhatChanged notification.")}
        }

        match melf_notifications.recv().await
        {
            Some(change_notice) =>  match change_notice {
                WhatChanged::GameEnded => {},
                _ => {panic!("Should have received game ended notification")}
            },
            None => { panic!("Should have produced a WhatChanged notification.")}
        }
    }


    #[tokio::test]
    pub async fn deleting_a_game_with_an_unknown_id_will_generate_no_matching_game()
    {
        let game_input_channel = init();
        let (game_sender, game_receiver) = channel::<Outcome>();

        add_new_game(&game_input_channel).await;

        let msg = Message { game_id: Uuid::new_v4(), reply_channel: game_sender, msg: Event::Delete };
        
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

        let msg = Message { game_id: game_id, reply_channel: game_sender, msg: Event::AddCharacter(character) };
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

        let msg = Message { game_id: Uuid::new_v4(), reply_channel: game_sender, msg: Event::AddCharacter(character) };
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

        let msg = Message { game_id: game_id, reply_channel: game_sender, msg: Event::StartCombat(combatants) };

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

        let msg = Message { game_id: game_id, reply_channel: game_sender, msg: Event::StartCombat(combatants) };

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

        let msg = Message { game_id: game_id, reply_channel: game_sender, msg: Event::StartCombat(Vec::<Uuid>::new()) };

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

        let msg = Message { game_id: Uuid::new_v4(), reply_channel: game_sender, msg: Event::StartCombat(combatants) };

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

        let msg = Message { game_id: game_id, reply_channel: game_sender, msg: Event::StartCombat(combatants) };

        let _response = game_input_channel.send(msg).await;

        let (game_sender, game_receiver) = channel::<Outcome>();

        let msg = Message { game_id: game_id, reply_channel: game_sender, msg: Event::BeginInitiativePhase };

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

        let msg = Message { game_id: game_id, reply_channel: game_sender, msg: Event::StartCombat(Vec::<Uuid>::new()) };

        let _response = game_input_channel.send(msg).await;

        let (game_sender, game_receiver) = channel::<Outcome>();

        let msg = Message { game_id: game_id, reply_channel: game_sender, msg: Event::BeginInitiativePhase };

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

        let msg = Message { game_id, reply_channel: game_sender, msg: Event::StartCombat(vec![character1, character2]) };

        assert!(game_input_channel.send(msg).await.is_ok());

        let (game_sender, _game_receiver) = channel::<Outcome>();
        let msg = Message { game_id, reply_channel: game_sender, msg: Event::BeginInitiativePhase };
        assert!(game_input_channel.send(msg).await.is_ok());

        let (game_sender, game_receiver) = channel::<Outcome>();
        let roll: Roll = Roll{ character_id: character1, roll: 13 };
        let msg = Message { game_id, reply_channel: game_sender, msg: Event::AddInitiativeRoll(roll) };
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

        let msg = Message { game_id, reply_channel: game_sender, msg: Event::StartCombat(combatants.clone()) };

        assert!(game_input_channel.send(msg).await.is_ok());

        let (game_sender, _game_receiver) = channel::<Outcome>();
        let msg = Message { game_id, reply_channel: game_sender, msg: Event::BeginInitiativePhase };
        assert!(game_input_channel.send(msg).await.is_ok());

        for character_id in combatants
        {
            let (game_sender, game_receiver) = channel::<Outcome>();
            let roll: Roll = Roll{character_id, roll: 13 };
            let msg = Message { game_id, reply_channel: game_sender, msg: Event::AddInitiativeRoll(roll) };
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

        let msg = Message { game_id, reply_channel: game_sender, msg: Event::StartCombat(combatants.clone()) };

        assert!(game_input_channel.send(msg).await.is_ok());

        let (game_sender, _game_receiver) = channel::<Outcome>();
        let msg = Message { game_id, reply_channel: game_sender, msg: Event::BeginInitiativePhase };
        assert!(game_input_channel.send(msg).await.is_ok());

        return (game_input_channel, game_id, combatants);
    }

    #[tokio::test]
    pub async fn sending_start_combat_round_before_all_combatants_have_sent_initiatives_generates_invalid_state_action()
    {
        let (game_input_channel, game_id, combatants) = construct_combat_ready_game().await;

        let (game_sender, _game_receiver) = channel::<Outcome>();
        let roll = Roll{ character_id: *combatants.get(0).unwrap(), roll: 23 };
        let msg = Message{game_id, reply_channel: game_sender, msg: Event::AddInitiativeRoll(roll)};
        assert!(game_input_channel.send(msg).await.is_ok());
        
        let (game_sender, game_receiver) = channel::<Outcome>();
        let msg = Message { game_id, reply_channel: game_sender, msg: Event::StartCombatRound };

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
        let msg = Message { game_id: Uuid::new_v4(), reply_channel: game_sender, msg: Event::New };

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
        let msg = Message { game_id, reply_channel: game_sender, msg: Event::StartCombatRound };

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
        let msg = Message { game_id, reply_channel: game_sender, msg: Event::StartCombatRound };

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
        let mut msg = Message { game_id: Uuid::new_v4(), reply_channel: game_sender, msg: Event::New };

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
        msg = Message { game_id, reply_channel: game_sender, msg: Event::BeginInitiativePhase };
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
        msg = Message { game_id, reply_channel: game_sender, msg: Event::StartCombat(vec![character1, character2]) };
        assert!(game_input_channel.send(msg).await.is_ok());
        assert!(game_receiver.await.is_ok());

        (game_sender, game_receiver) = channel::<Outcome>();

        msg = Message { game_id, reply_channel: game_sender, msg: Event::BeginInitiativePhase };
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
        msg = Message{game_id, reply_channel: game_sender, msg: Event::BeginInitiativePhase};
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
        msg = Message { game_id, reply_channel: game_sender, msg: Event::AddInitiativeRoll(Roll { character_id: character1, roll: 13 })};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { game_id, reply_channel: game_sender, msg: Event::AddInitiativeRoll(Roll { character_id: character2, roll: 23 })};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { game_id, reply_channel: game_sender, msg: Event::StartCombatRound};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { game_id, reply_channel: game_sender, msg: Event::TakeAction(Action { character_id: character2, action: ActionType::Complex })};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { game_id, reply_channel: game_sender, msg: Event::AdvanceTurn};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { game_id, reply_channel: game_sender, msg: Event::TakeAction(Action { character_id: character1, action: ActionType::Complex })};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, _game_receiver) = channel::<Outcome>();
        msg = Message { game_id, reply_channel: game_sender, msg: Event::AdvanceTurn};
        assert!(game_input_channel.send(msg).await.is_ok());

        (game_sender, game_receiver) = channel::<Outcome>();
        msg = Message{game_id, reply_channel: game_sender, msg: Event::BeginInitiativePhase};
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
        let mut msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::AddInitiativeRoll
            (Roll{ character_id: *characters.get(0).unwrap(), roll: 13 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::AddInitiativeRoll
            (Roll{ character_id: *characters.get(1).unwrap(), roll: 23 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::AddInitiativeRoll
            (Roll{ character_id: *characters.get(2).unwrap(), roll: 9 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::AddInitiativeRoll
            (Roll{ character_id: *characters.get(3).unwrap(), roll: 16 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());

        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::StartCombatRound};
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::TakeAction
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
        let mut msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::AddInitiativeRoll
            (Roll{ character_id: *characters.get(0).unwrap(), roll: 13 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::AddInitiativeRoll
            (Roll{ character_id: *characters.get(1).unwrap(), roll: 23 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::AddInitiativeRoll
            (Roll{ character_id: *characters.get(2).unwrap(), roll: 9 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::AddInitiativeRoll
            (Roll{ character_id: *characters.get(3).unwrap(), roll: 16 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());

        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::StartCombatRound};
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());

        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::TakeAction(Action{ character_id: *characters.get(2).unwrap(), action: ActionType::Free })};
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
        let mut msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::AddInitiativeRoll
            (Roll{ character_id: *characters.get(0).unwrap(), roll: 13 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::AddInitiativeRoll
            (Roll{ character_id: *characters.get(1).unwrap(), roll: 23 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::AddInitiativeRoll
            (Roll{ character_id: *characters.get(2).unwrap(), roll: 9 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());
        
        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::AddInitiativeRoll
            (Roll{ character_id: *characters.get(3).unwrap(), roll: 16 }) };
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());

        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::StartCombatRound};
        assert!(sender.send(msg).await.is_ok());
        assert!(our_receiver.await.is_ok());

        (game_owned_sender, our_receiver) = channel::<Outcome>();
        msg = Message{ game_id, reply_channel: game_owned_sender, msg: Event::TakeAction
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