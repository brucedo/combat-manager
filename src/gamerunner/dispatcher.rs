use std::{collections::{HashSet, HashMap}, sync::Arc};

use tokio::sync::mpsc::{channel, Receiver};
use tokio::sync::oneshot::Sender as OneShotSender;
use log::{debug, error};
use uuid::Uuid;

use crate::tracker::{game::{Game, ActionType}, character::Character};

use super::{registry::GameRegistry, GameId, ErrorKind, Error, PlayerId, WhatChanged, authority::{Authority, Role}, CharacterId};

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
    JoinGame,
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
    let request = authority.request();
    match request
    {
        Request::NewPlayer => {
            debug!("Request is to register as a player.");
            (register_player(authority, registry), None)
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
        Request::JoinGame => {
            debug!("Request is to let a player join a game.");
            join_game(authority, registry)
        },
        Request::AddCharacter(character) => {
            debug!("Request is to add a new character.");
            (add_character(character, registry, authority), None)
        },
        Request::GetFullCast => {
            debug!("Request is to get the full cast list.");
            (get_full_cast(registry, authority), None)
        },
        Request::GetNpcCast => {
            debug!("Request is to get the NPC cast list.");
            (get_npcs(registry, authority), None)
        },
        Request::GetPcCast => {
            debug!("Reqeust is to get the PC cast list.");
            (get_pcs(registry, authority), None)
        }
        Request::GetCharacter(id) => {
            debug!("Request is to get a character by id.");
            (get_char(id, registry, authority), None)
        }
        Request::StartCombat(combatants) => {
            debug!("Request is to start the combat phase.");
            (start_combat(registry, combatants.to_owned(), authority), None)

        },
        Request::AddInitiativeRoll(roll) => {
            debug!("Request is to add an initiative roll.");
            (add_init_roll(roll, authority, registry), None)
        },
        Request::BeginInitiativePhase => {
            debug!("Request is to begin the initiative phase.");
            (try_initiative_phase(registry, authority), None)
        },
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

fn register_player(authority: &Authority, player_directory: &mut GameRegistry) -> Outcome
{
    match authority.resource_role() 
    {
        Role::RoleUnregistered => {
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
        },
        _ => {
            Outcome::Error(Error { message: String::from("Player is already registered."), kind: ErrorKind::InvalidStateAction })
        }
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

    match authority.resource_role()
    {
        Role::RoleUnregistered => {
            Outcome::Error(Error {message: String::from("User must be registered before a game may be created."), kind: ErrorKind::InvalidStateAction})
        },
        Role::RoleRegistered(player_id) | Role::RolePlayer(player_id, _) | Role::RoleGM(player_id, _) | Role::RoleObserver(player_id, _) => {
            let game_id = Uuid::new_v4();
            running_games.new_game(*player_id, game_id, Game::new());
            Outcome::Created(game_id)
        }
    }

}

fn end_game(authority: &Authority, directory: &mut GameRegistry) -> (Outcome, Option<HashSet<Uuid>>)
{

    match authority.resource_role()
    {
        Role::RoleGM(player_id, game_id) => 
        {
            match directory.delete_game(*game_id)
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
                }
            }
        }
        _ => 
        {
            (Outcome::Error(Error { message: String::from("The action requested (Delete Game) may only be initiated by the game's GM."), kind: ErrorKind::NotGameOwner }), None)
        }
    }
    
}

fn join_game(authority: &Authority, game_directory: &mut GameRegistry) -> (Outcome, Option<HashSet<PlayerId>>)
{
    
    match authority.resource_role()
    {
        Role::RoleGM(player_id, game_id) | Role::RolePlayer(player_id, game_id) | Role::RoleObserver(player_id, game_id) => 
        {
            match game_directory.join_game(*player_id, *game_id)
            {
                Ok(_) => {
                    (Outcome::JoinedGame(GameState { for_player:  *player_id }), None)
                },
                Err(_) => {
                    (Outcome::Error(Error { message: String::from(format!("No matching game for id {}", game_id)), kind: ErrorKind::NoMatchingGame }), None)
                },
            }
            
        },
        Role::RoleUnregistered | Role::RoleRegistered(_) =>
        {
            (Outcome::Error(Error { message: String::from("User must be registered or provide the game ID before they may join a game."), kind: ErrorKind::UnknownId }), None)
        }
    }
}

// fn find_game_and_act<F>(authority: &Authority, running_games: &mut GameRegistry, action: F) -> (Outcome, Option<HashSet<PlayerId>>)
// where
//     F: FnOnce(&mut Game, &Authority) -> Outcome
// {
//     let response: Outcome;
    
//     if let Some(game_id) = authority.game_id()
//     {
//         match running_games.get_mut_game(game_id)
//         {
//             Some(mut game) => 
//             {
//                 response = action(&mut game, authority);
//             },
//             None => {response = game_not_found(game_id)},
//         }
//     }
//     else
//     {
//         response = Outcome::Error(Error {message: String::from("Game ID field left empty - action cannot be taken."), kind: ErrorKind::InvalidStateAction})
//     }

//     return (response, None);
// }

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

fn add_character(character: &Character, registry: &mut GameRegistry, authority: &Authority) -> Outcome
{
    match authority.resource_role()
    {
        Role::RolePlayer(player_id, game_id) | Role::RoleGM(player_id, game_id) => {
            if let Some(game) = registry.get_mut_game(game_id)
            {
                let char_id = game.add_cast_member((*character).clone());
                return Outcome::CharacterAdded((*game_id, char_id));
            }
            else
            {
                Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::UnknownId})
            }
            
        }, 
        _ => {
            return Outcome::Error(Error { message: String::from("Observers may not create characters in a game."), kind: ErrorKind::InvalidStateAction })
        }
    }
    
}

fn get_full_cast(registry: &mut GameRegistry, authority: &Authority) -> Outcome
{
    match authority.resource_role()
    {
        Role::RoleGM(_, game_id) => {
            if let Some(game) = registry.get_game(game_id)
            {
                Outcome::CastList(game.get_cast())
            }
            else
            {
                Outcome::Error(Error { message: String::from("The game identifier provided does not resolve to a running game."), kind: ErrorKind::UnknownId})
            }
        }
        _ => Outcome::Error(Error { message: String::from("Only GMs may request the full character roster."), kind: ErrorKind::InvalidStateAction })
    }
    
}

fn get_npcs(registry: &mut GameRegistry, authority: &Authority) -> Outcome
{
    match authority.resource_role() 
    {
        Role::RoleGM(_, game_id) => {
            if let Some(game) = registry.get_game(game_id)
            {
                Outcome::CastList(game.get_npcs())
            }
            else
            {
                Outcome::Error( Error { message: String::from("The game identifier provided does not resolve to a running game."), kind: ErrorKind::UnknownId})
            }
        }
        _ => Outcome::Error(Error {message: String::from("Only GMs may request the NPC character roster."), kind: ErrorKind::InvalidStateAction })
    }
    
}

fn get_pcs(registry: &mut GameRegistry, authority: &Authority) -> Outcome
{
    match authority.resource_role()
    {
        Role::RoleGM(_, game_id) | Role::RolePlayer(_, game_id) => {
            if let Some(game) = registry.get_game(game_id)
            {
                Outcome::CastList(game.get_pcs())
            }
            else
            {
                Outcome::Error( Error { message: String::from("The game identifier provided does not resolve to a running game."), kind: ErrorKind::UnknownId})
            }
        }
        _ => Outcome::Error(Error {message: String::from("Only active participants in the game may get the player roster."), kind: ErrorKind::InvalidStateAction })
    }
    
}

fn get_char(char_id: &CharacterId, registry: &GameRegistry, authority: &Authority) -> Outcome
{
    

    match authority.resource_role()
    {
        Role::RolePlayer(player_id, game_id) =>
        {
            match registry.get_game(&game_id)
            {
                Some(game) => {
                    if registry.player_chars(&game_id, &player_id).map_or(false, |chars| chars.contains(&char_id))
                    {
                        return Outcome::Found(game.get_cast_by_id(&char_id));
                    }
                    else
                    {
                        return Outcome::Error(Error { message: String::from("Player ID is not an owner of the character."), kind: ErrorKind::UnknownId });
                    }
                },
                None =>
                {
                    Outcome::Error(Error { message: String::from("Provided ID does not map to a running game."), kind: ErrorKind::UnknownId })
                }
            }
        }
        Role::RoleGM(_, game_id) =>
        {
            match registry.get_game(&game_id)
            {
                Some(game) => {Outcome::Found(game.get_cast_by_id(&char_id))}
                None => {Outcome::Error(Error { message: String::from("Provided ID does not map to a running game."), kind: ErrorKind::UnknownId })}
            }
        }
        _ =>
        {
            Outcome::Error(Error{ message: String::from("Cannot get character for a game or player that does not exist."), kind: ErrorKind::NotGamePlayer })
        }
    }
}

fn start_combat(game_registry: &mut GameRegistry, combatants: Vec<CharacterId>, authority: &Authority) -> Outcome
{

    let response: Outcome;

    match authority.resource_role()
    {
        Role::RoleGM(_, game_id) => {
            if let Some(game) = game_registry.get_mut_game(game_id)
            {
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
            }
            else
            {
                response = Outcome::Error(Error { message: String::from("Provided ID does not map to a running game."), kind: ErrorKind::UnknownId});
            }
        },
        _ => {response = Outcome::Error(Error { message: String::from("Only the Game GM may initiate combat."), kind: ErrorKind::NotGameOwner })}
    }

    return response;

}

fn try_initiative_phase(registry: &mut GameRegistry, authority: &Authority) -> Outcome
{
    let response: Outcome;

    match authority.resource_role()
    {
        Role::RoleGM(_, game_id) => {
            if let Some(game) = registry.get_mut_game(game_id)
            {
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
            }
            else 
            {
                response = Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::NoMatchingGame});
            }
        },
        _ => {
            response = Outcome::Error(Error {message: String::from("Only the GM may begin initiative."), kind: ErrorKind::UnauthorizedAction})
        }
    }
    

    return response;
}

// fn add_init_roll(character_id: Uuid, roll: i8, game: &mut Game) -> Outcome
fn add_init_roll(roll: &Roll, authority: &Authority, registry: &GameRegistry) -> Outcome
{
    // let response: Outcome;

    // let (character_id, roll) = (roll.character_id, roll.roll);

    // let game: &mut Game;

    // if authority.game_id().is_none()
    // {
        return Outcome::Error(Error { message: String::from("No game found by provided ID."), kind: ErrorKind::UnknownId })
    // }
    
    // game = registry.get_mut_game(authority.game_id().unwrap());

    // match (authority.resource_role(), authority.game_id(), authority.player_id())
    // {
    //     (Role::RoleGM, Some(game_id), _) => {
    //         if let Some(game) = registry.get_mut_game(game_id)
    //         {
    //             if let Err(error) = game.accept_initiative_roll(character_id, roll)
    //             {

    //             }
    //         }
    //     },
    //     (Role::RolePlayer, Some(game_id), Some(player_id)) => {

    //     }
    //     _ => {}
    // }
    

    // if let Err(result) = game.accept_initiative_roll(roll.character_id, roll.roll)
    // {
    //     match result.kind
    //     {
    //         crate::tracker::game::ErrorKind::InvalidStateAction => {
    //             response = Outcome::Error
    //             (
    //                 Error 
    //                 { 
    //                     message: String::from(format!("The game is not in the correct state to take initiative rolls.")), 
    //                     kind: ErrorKind::InvalidStateAction 
    //                 }
    //             );
    //         },
    //         crate::tracker::game::ErrorKind::UnknownCastId => {
    //             response = Outcome::Error
    //             (
    //                 Error 
    //                 { 
    //                     message: String::from(format!("Character ID does not exist: {}", result.msg)), 
    //                     kind: ErrorKind::NoMatchingGame 
    //                 }
    //             );
    //         },
    //         _ => {unreachable!()},
    //     }
    // }
    // else
    // {
    //     response = Outcome::InitiativeRollAdded;
    // }

    // return response;

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
