use std::{collections::{HashSet, HashMap}, sync::Arc};

use tokio::sync::mpsc::{channel, Receiver};
use tokio::sync::oneshot::Sender as OneShotSender;
use log::{debug, error};
use uuid::Uuid;

use crate::tracker::{game::{Game, ActionType}, character::Character};

use super::{registry::GameRegistry, GameId, ErrorKind, Error, PlayerId, WhatChanged, authority::{Authority, Role}};

pub struct Message
{
    pub game_id: Option<GameId>,
    pub player_id: Option<PlayerId>,
    pub reply_channel: OneShotSender<Outcome>,
    pub msg: Request,
}

pub enum Request
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
    CharacterAdded((GameId, Uuid)),
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

pub struct InitiativeState
{
    pub waiting: bool,
    pub remaining: Vec<Uuid>
}

pub struct Roll
{
    pub character_id: Uuid,
    pub roll: i8,
}

pub struct Action
{
    pub character_id: Uuid,
    pub action: ActionType
}

pub struct NewPlayer
{
    pub player_id: Uuid,
    pub player_1_receiver: Receiver<WhatChanged>
}

pub struct GameState
{
    pub for_player: Uuid,
}

pub fn dispatch_message(registry: &mut GameRegistry, authority: &Authority) -> (Outcome, Option<HashSet<Uuid>>)
{
    let (player_id, game_id, request) = (authority.player_id(), authority.game_id(), authority.request());
    match request
    {
        Request::NewPlayer => {
            debug!("Request is to register as a player.");
            (register_player(registry), None)
        },
        Request::Enumerate => {
            debug!("Request is for a list of running games.");
            (enumerate(registry), None)
        }
        Request::New => {
            debug!("Request is for new game.");
            (new_game(authority, registry), None)
        },
        Request::Delete => {
            debug!("Request is to remove game.");
            end_game(authority, registry)
        },
        Request::JoinGame(player_id) => {
            debug!("Request is to let a player join a game.");
            join_game(authority, registry)
        },
        Request::AddCharacter(character) => {
            debug!("Request is to add a new character.");
            find_game_and_act(authority, registry, | game, authority | {add_character(character, game, authority)})
        },
        // Request::GetFullCast => {
        //     debug!("Request is to get the full cast list.");
        //     find_game_and_act(registry, game_id, get_full_cast)
        // },
        // Request::GetNpcCast => {
        //     debug!("Request is to get the NPC cast list.");
        //     find_game_and_act(registry, game_id, get_npcs)
        // },
        // Request::GetPcCast => {
        //     debug!("Reqeust is to get the PC cast list.");
        //     find_game_and_act(registry, game_id, get_pcs)
        // }
        // Request::GetCharacter(id) => {
        //     debug!("Request is to get a character by id.");
        //     find_game_and_act(registry, game_id, |game| {get_char(id, game)})
        // }
        // Request::StartCombat(combatants) => {
        //     debug!("Request is to start the combat phase.");                
        //     find_game_and_act(registry, game_id, | game | {start_combat(combatants.to_owned(), game)})
        // },
        // Request::AddInitiativeRoll(roll) => {
        //     debug!("Request is to add an initiative roll.");
        //     find_game_and_act(registry, game_id, | game | { add_init_roll(roll, game)})
        // },
        // Request::BeginInitiativePhase => {
        //     debug!("Request is to begin the initiative phase.");
        //     find_game_and_act(registry, game_id, try_initiative_phase)
        // },
        // Request::StartCombatRound => {
        //     debug!("Request is to begin a combat round.");
        //     find_game_and_act( registry, game_id, try_begin_combat)
        // },
        // Request::TakeAction(action) =>
        // {
        //     debug!("Request is for some character to perform some action.");
        //     find_game_and_act( registry, game_id, | game | {take_action(game, action)})
        // }
        // Request::AdvanceTurn => {
        //     debug!("Request is to advance to the next event in the pass.");
        //     find_game_and_act( registry, game_id, try_advance_turn)
        // }
        // Request::WhoGoesThisTurn => {
        //     debug!("Request is to see who is going this turn.");
        //     find_game_and_act(registry, game_id, list_current_turn_events)
        // }
        // Request::WhatHasYetToHappenThisTurn => {
        //     debug!("Request is to see who has yet to go.");
        //     find_game_and_act(registry, game_id, list_unresolved_events)
        // }
        // Request::WhatHappensNextTurn => {
        //     debug!("Request is to see what happens next turn.");
        //     find_game_and_act(registry, game_id, list_next_turn_events)
        // }
        // Request::AllEventsThisPass => {
        //     debug!("Request is for a full accounting of all events on this pass.");
        //     find_game_and_act(registry, game_id, list_all_events_by_id_this_pass)
        // }
        // Request::NextInitiative => {
        //     debug!("Request is to get the next initiative number.");
        //     find_game_and_act(registry, game_id, next_initiative)
        // }
        // Request::CurrentInitiative => {
        //     debug!("Request is to get the current initiative number.");
        //     find_game_and_act(registry, game_id, current_initiative)
        // }
        // Request::AllRemainingInitiatives => {
        //     debug!("Request is to get any initiatives that have not been fully resolved.");
        //     find_game_and_act(registry, game_id, remaining_initiatives_are)
        // }
        _ => {todo!()}
    }
}

fn register_player(player_directory: &mut GameRegistry) -> Outcome
{
    let mut player_id = Uuid::new_v4();

    while player_directory.is_registered(&player_id)
    {
        player_id = Uuid::new_v4();
    }

    let (player_sender, player_receiver) = channel(32);
    let player_info = NewPlayer{ player_id, player_1_receiver: player_receiver };   

    match player_directory.register_player(player_id, player_sender)
    {
        Ok(_) => {Outcome::NewPlayer(player_info)},
        Err(_) => {unreachable!("Duplicate ID encountered despite explicitly checking for duplicate ID before joining")}
    }

    // return Outcome::NewPlayer(player_info);
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

fn new_game(authority: &Authority, running_games: &mut GameRegistry) -> Outcome
{
    let response: Outcome;

    if let Some(player_id) = authority.player_id()
    {
        let game_id = Uuid::new_v4();
        running_games.new_game(player_id, game_id, Game::new());
        response = Outcome::Created(game_id);
    }
    else
    {
        response = Outcome::Error(Error { message: String::from("Player ID field was left blank."), kind: ErrorKind::InvalidStateAction})
    }
    return response;
}

fn end_game(authority: &Authority, directory: &mut GameRegistry) -> (Outcome, Option<HashSet<Uuid>>)
{
    if *authority.resource_role() != Role::RoleGM
    {
        (Outcome::Error(Error { message: String::from("The action requested (Delete Game) may only be initiated by the game's GM."), kind: ErrorKind::NotGameOwner }), 
        None)
    }
    else if let Some(game_id) = authority.game_id()
    {

        match directory.delete_game(game_id)
        {
            Ok(game_entry) => 
            {
                let to_notify = game_entry.players;
                // let to_notify = directory.players_by_game(game);
                (Outcome::Destroyed, Some(to_notify))
            },
            Err(_) => 
            {
                (Outcome::Error(
                Error{ message: String::from(format!("No game by ID {} exists.", game_id)), kind: ErrorKind::NoMatchingGame }), None)
            },
        }
    }
    else {
        (Outcome::Error(Error { message: String::from("This branch probably should not be accessible while ending a game."), kind: ErrorKind::InvalidStateAction }), None)
    }
}

fn join_game(authority: &Authority, game_directory: &mut GameRegistry) -> (Outcome, Option<HashSet<PlayerId>>)
{

    if let (Some(player_id), Some(game_id)) = (authority.player_id(), authority.game_id())
    {
        match game_directory.join_game(player_id, game_id)
        {
            Ok(_) => 
            {   
                // let to_notify = game_directory.players_by_game(game_id);
                (Outcome::JoinedGame(GameState {for_player: player_id}), None)
            },
            Err(_) => 
            {
                (Outcome::Error(Error { message: String::from(format!("No matching game for id {}", game_id)), kind: ErrorKind::NoMatchingGame }), None)
            },
        }
    }
    else
    {
        (Outcome::Error(Error { message: String::from("One or both of the player id and game id were empty."), kind: ErrorKind::InvalidStateAction}), None)
    }
}

fn find_game_and_act<F>(authority: &Authority, running_games: &mut GameRegistry, action: F) -> (Outcome, Option<HashSet<PlayerId>>)
where
    F: FnOnce(&mut Game, &Authority) -> Outcome
{
    let response: Outcome;
    
    if let Some(game_id) = authority.game_id()
    {
        match running_games.get_mut_game(game_id)
        {
            Some(mut game) => 
            {
                response = action(&mut game, authority);
            },
            None => {response = game_not_found(game_id)},
        }
    }
    else
    {
        response = Outcome::Error(Error {message: String::from("Game ID field left empty - action cannot be taken."), kind: ErrorKind::InvalidStateAction})
    }

    return (response, None);
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

fn add_character(character: &Character, game: &mut Game, authority: &Authority) -> Outcome
{
    match authority.resource_role()
    {
        Role::RolePlayer | Role::RoleGM => {
            let game_id = authority.game_id().unwrap();
            let char_id = game.add_cast_member((*character).clone());
            return Outcome::CharacterAdded((game_id, char_id));
        }, 
        _ => {
            return Outcome::Error(Error { message: String::from("Observers may not create characters in a game."), kind: ErrorKind::InvalidStateAction })
        }
    }
    
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
    Outcome::Found(game.get_cast_by_id(&char_id))
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
