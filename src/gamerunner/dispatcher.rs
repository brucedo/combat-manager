use std::{collections::{HashSet, HashMap}, sync::Arc};

use rocket::http::hyper::request;
use tokio::sync::mpsc::{channel, Sender, Receiver};
use tokio::sync::oneshot::Sender as OneShotSender;
use log::{debug, error};
use uuid::Uuid;

use crate::{tracker::{game::{Game, ActionType, GameError, ErrorKind as GameErrorKind}, character::Character}};

use super::{registry::GameRegistry, GameId, ErrorKind, Error, PlayerId, WhatChanged, authority::{Authority, Role}, CharacterId, notifier::{Notification, PlayerJoined, NewCharacter}};

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
    pub player_1_receiver: Receiver<Arc<WhatChanged>>
}

pub struct GameState
{
    pub for_player: Uuid,
}

pub fn dispatch_message2(registry: &mut GameRegistry, authority: &Authority) -> (Outcome, Option<Notification>)
{
    let request = authority.request();

    match request
    {
        Request::NewPlayer => {
            debug!("Request is to register as a player.");
            register_player2(authority, registry)
        }
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
            end_game2(authority, registry)
        },
        Request::JoinGame => {
            debug!("Request is to let a player join a game.");
            join_game2(authority, registry)
        },
        Request::AddCharacter(character) => {
            debug!("Request is to add a new character.");
            add_character2(character, registry, authority)
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
            start_combat2(registry, combatants.to_owned(), authority)

        },
        Request::AddInitiativeRoll(roll) => {
            debug!("Request is to add an initiative roll.");
            add_init_roll2(roll, authority, registry)
        },
        Request::BeginInitiativePhase => {
            debug!("Request is to begin the initiative phase.");
            try_initiative_phase2(registry, authority)
        },
        Request::StartCombatRound => {
            debug!("Request is to begin a combat round.");
            try_begin_combat2( registry, authority)
        },
        Request::TakeAction(action) =>
        {
            debug!("Request is for some character to perform some action.");
            take_action2( registry, action, authority)
        }
        Request::AdvanceTurn => {
            debug!("Request is to advance to the next event in the pass.");
            try_advance_turn2( registry, authority)
        }
        Request::WhoGoesThisTurn => {
            debug!("Request is to see who is going this turn.");
            (list_current_turn_events(registry, authority), None)
        }
        Request::WhatHasYetToHappenThisTurn => {
            debug!("Request is to see who has yet to go.");
            (list_unresolved_events(registry, authority), None)
        }
        Request::WhatHappensNextTurn => {
            debug!("Request is to see what happens next turn.");
            (list_next_turn_events(registry, authority), None)
        }
        Request::AllEventsThisPass => {
            debug!("Request is for a full accounting of all events on this pass.");
            (list_all_events_by_id_this_pass(registry, authority), None)
        }
        Request::NextInitiative => {
            debug!("Request is to get the next initiative number.");
            (next_initiative(registry, authority), None)
        }
        Request::CurrentInitiative => {
            debug!("Request is to get the current initiative number.");
            (current_initiative(registry, authority), None)
        }
        Request::AllRemainingInitiatives => {
            debug!("Request is to get any initiatives that have not been fully resolved.");
            (remaining_initiatives_are(registry, authority), None)
        }
        _ => (Outcome::Error(Error { message: String::from("Not Yet Implemented"), kind: ErrorKind::InvalidStateAction }), None)
    }
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
        Request::StartCombatRound => {
            debug!("Request is to begin a combat round.");
            (try_begin_combat( registry, authority), None)
        },
        Request::TakeAction(action) =>
        {
            debug!("Request is for some character to perform some action.");
            (take_action( registry, action, authority), None)
        }
        Request::AdvanceTurn => {
            debug!("Request is to advance to the next event in the pass.");
            (try_advance_turn( registry, authority), None)
        }
        Request::WhoGoesThisTurn => {
            debug!("Request is to see who is going this turn.");
            (list_current_turn_events(registry, authority), None)
        }
        Request::WhatHasYetToHappenThisTurn => {
            debug!("Request is to see who has yet to go.");
            (list_unresolved_events(registry, authority), None)
        }
        Request::WhatHappensNextTurn => {
            debug!("Request is to see what happens next turn.");
            (list_next_turn_events(registry, authority), None)
        }
        Request::AllEventsThisPass => {
            debug!("Request is for a full accounting of all events on this pass.");
            (list_all_events_by_id_this_pass(registry, authority), None)
        }
        Request::NextInitiative => {
            debug!("Request is to get the next initiative number.");
            (next_initiative(registry, authority), None)
        }
        Request::CurrentInitiative => {
            debug!("Request is to get the current initiative number.");
            (current_initiative(registry, authority), None)
        }
        Request::AllRemainingInitiatives => {
            debug!("Request is to get any initiatives that have not been fully resolved.");
            (remaining_initiatives_are(registry, authority), None)
        }
        _ => {todo!()}
    }
}

fn register_player2(authority: &Authority, player_directory: &mut GameRegistry) -> (Outcome, Option<Notification>)
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
                Ok(_) => {(Outcome::NewPlayer(player_info), None)},
                Err(_) => {unreachable!("Duplicate ID encountered despite explicitly checking for duplicate ID before joining")}
            }
        },
        _ => {
            (Outcome::Error(Error { message: String::from("Player is already registered."), kind: ErrorKind::InvalidStateAction }), None)
        }
    }
    

    // return Outcome::NewPlayer(player_info);
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

fn end_game2(authority: &Authority, directory: &mut GameRegistry) -> (Outcome, Option<Notification>)
{

    match authority.resource_role()
    {
        Role::RoleGM(_player_id, game_id) => 
        {
            match directory.delete_game(*game_id)
            {
                Ok(game_entry) => 
                {
                    let to_notify = game_entry.players;
                    let senders: Vec<Sender<Arc<WhatChanged>>> = to_notify.iter()
                        .map(|player_id| directory.get_player_sender(player_id))
                        .filter(|opt| opt.is_some())
                        .map(|vec| vec.unwrap())
                        .collect();
                    let notification = Notification { change_type: Arc::from(WhatChanged::GameEnded), send_to: senders };
                    // let to_notify = directory.players_by_game(game);
                    (Outcome::Destroyed, Some(notification))
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


fn join_game2(authority: &Authority, game_directory: &mut GameRegistry) -> (Outcome, Option<Notification>)
{
    
    match authority.resource_role()
    {
        Role::RoleGM(player_id, game_id) | Role::RolePlayer(player_id, game_id) | Role::RoleObserver(player_id, game_id) => 
        {
            // We could alternatively get the list of players after we successfully join the game.  However, that means that the retrieved player list 
            // includes the ID of the player who just joined, and we are sending an action Outcome to them - we don't need to send a Notification too.
            // So we'd need to add a filter step to get the list without the just-added player.  Not sure this is much better....
            let other_players = game_directory.players_by_game(game_id); 
            let opt_senders: Option<Vec<Sender<Arc<WhatChanged>>>> = 
                other_players.map(
                    |opt| opt.iter().map(|id| game_directory.get_player_sender(id))
                    .filter(|opt| opt.is_some()).map(|opt| opt.unwrap())
                    .collect::<Vec<Sender<Arc<WhatChanged>>>>()
                );

            if game_directory.join_game(*player_id, *game_id).is_ok()
            {
                let notification = match opt_senders 
                {
                    Some(senders) => {
                        Some(Notification{ change_type: Arc::from(WhatChanged::NewPlayer(PlayerJoined { name: String::from(""), 
                        player_id: *player_id })), send_to: senders})
                    }, 
                    None => None
                };
                (Outcome::JoinedGame(GameState { for_player:  *player_id }), notification)
            }
            else {
                (Outcome::Error(Error { message: String::from(format!("No matching game for id {}", game_id)), kind: ErrorKind::NoMatchingGame }), None)
            }
            
        },
        Role::RoleUnregistered | Role::RoleRegistered(_) =>
        {
            (Outcome::Error(Error { message: String::from("User must be registered or provide the game ID before they may join a game."), kind: ErrorKind::UnknownId }), None)
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


fn add_character2(character: &Character, registry: &mut GameRegistry, authority: &Authority) -> (Outcome, Option<Notification>)
{
    match authority.resource_role()
    {
        Role::RolePlayer(player_id, game_id) | Role::RoleGM(player_id, game_id) => {
            let senders = registry.players_by_game(game_id).map(|hs| hs.iter()
                    .map(|player_id| registry.get_player_sender(player_id)).filter(|opt| opt.is_some())
                    .map(|opt| opt.unwrap()).collect::<Vec<Sender<Arc<WhatChanged>>>>());

            if let Some(char_id) = registry.add_character(player_id, game_id, character.clone())
            {
                let notification = match senders
                {
                    Some(sender_list) => {
                        Some(
                        Notification{ change_type: Arc::from(WhatChanged::NewCharacter(NewCharacter{ player_id: *player_id, character_id: char_id, metatype: character.metatype })), 
                        send_to: sender_list })
                    },
                    None => {None}
                };

                (Outcome::CharacterAdded((*game_id, char_id)), notification)
            }
            else 
            {
                (Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::UnknownId}), None)
            }
        }, 
        _ => {
            return (Outcome::Error(Error { message: String::from("Observers may not create characters in a game."), kind: ErrorKind::InvalidStateAction }), None)
        }
    }
    
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
                    if registry.characters_by_player(&game_id, &player_id).map_or(false, |chars| chars.contains(&char_id))
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

fn start_combat2(game_registry: &mut GameRegistry, combatants: Vec<CharacterId>, authority: &Authority) -> (Outcome, Option<Notification>)
{

    let response: Outcome;

    match authority.resource_role()
    {
        Role::RoleGM(_, game_id) => {

            let notify_list = combatants.iter().map(|char_id| game_registry.players_by_character(game_id, char_id))
                .filter(|p| p.is_some()).map(|p| *p.unwrap()).collect::<Vec<PlayerId>>();
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

    return (response, None);

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

fn try_initiative_phase2(registry: &mut GameRegistry, authority: &Authority) -> (Outcome, Option<Notification>)
{
    match authority.resource_role()
    {
        Role::RoleGM(_, game_id) => {
            if let Some(game) = registry.get_mut_game(game_id)
            {
                match game.start_initiative_phase()
                {
                    Ok(_) => {
                        let combat_chararcters = game.get_combatants();
                        let senders = combat_chararcters.iter()
                            .map(|char_id| registry.players_by_character(game_id, char_id))
                            .filter(|player_id_opt| player_id_opt.is_some())
                            .map(|player_id_opt| player_id_opt.unwrap())
                            .map(|player_id| registry.get_player_sender(player_id))
                            .map(|player_sender_opt| player_sender_opt.unwrap())
                            .collect::<Vec<Sender<Arc<WhatChanged>>>>();
                        
                        debug!("Non-error returned from game.start_initiative_phase()");
                        (Outcome::InitiativePhaseStarted, Some(Notification { change_type: Arc::from(WhatChanged::StartingInitiativePhase), send_to: senders }))
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
                        (Outcome::Error(runner_err), None)
                    },
                }
            }
            else 
            {
                (Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::NoMatchingGame}), None)
            }
        },
        _ => {
            (Outcome::Error(Error {message: String::from("Only the GM may begin initiative."), kind: ErrorKind::UnauthorizedAction}), None)
        }
    }
    
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

fn add_init_roll2(roll: &Roll, authority: &Authority, registry: &mut GameRegistry) -> (Outcome, Option<Notification>)
{

    match authority.resource_role() 
    {
        Role::RoleGM(player_id, game_id) => 
        {
            if let Some(game) = registry.get_mut_game(game_id)
            {
                match game.accept_initiative_roll(roll.character_id, roll.roll)
                {
                    Ok(_) => {(Outcome::InitiativeRollAdded, None)},
                    Err(GameError{kind: GameErrorKind::InvalidStateAction, ..}) => {
                        (Outcome::Error(Error {message: String::from("The game is not in the initiatve state."), kind: ErrorKind::InvalidStateAction}), None)
                    }
                    Err(GameError{kind: GameErrorKind::UnknownCastId, ..}) => {
                        (Outcome::Error(Error { message: String::from("The character ID provided is not registered as part of combat."), kind: ErrorKind::UnknownId }), None)
                    }
                    _ => {
                        (Outcome::Error(Error { message: String::from("Unexpected error type returned from initiative add."), kind: ErrorKind::InvalidStateAction}), None)
                    }
                }

                
            }
            else
            {
                return (Outcome::Error(Error { message: String::from("No game found by provided ID."), kind: ErrorKind::UnknownId }), None)
            }
        },
        Role::RolePlayer(_, _) => todo!(),
        _ => (Outcome::Error(Error { message: String::from("Only players and the GM may roll for initiative."), kind: ErrorKind::UnauthorizedAction}), None)
    }

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

fn try_begin_combat2(registry: &mut GameRegistry, authority: &Authority) -> (Outcome, Option<Notification>)
{
    
    match authority.resource_role()
    {
        Role::RoleGM(_, game_id) => {
            

            let Some(game) = registry.get_mut_game(game_id) 
            else {return (Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::NoMatchingGame}), None)};
            if let Err(err) = game.start_combat_rounds()
            {
                match err.kind
                {
                    crate::tracker::game::ErrorKind::InvalidStateAction => {
                        (Outcome::Error(Error{ message: err.msg, kind: ErrorKind::InvalidStateAction }), None)
                    },
                    _ => {unreachable!()}
                }
            }
            else 
            {
                let senders = game.get_combatants().iter().map(|char_id| registry.players_by_character(game_id, char_id))
                    .filter(|player_id| player_id.is_some()).map(|player_id| player_id.unwrap())
                    .map(|player_id| registry.get_player_sender(player_id)).map(|sender| sender.unwrap())
                    .collect::<Vec<Sender<Arc<WhatChanged>>>>();
                (Outcome::CombatRoundStarted, Some(Notification{ change_type: Arc::from(WhatChanged::CombatStarted), send_to: senders }))
            }
        }
        _ => (Outcome::Error(Error {message: String::from("Only the game's GM may initiate combat."), kind: ErrorKind::UnauthorizedAction}), None)
    }
}

fn try_begin_combat(registry: &mut GameRegistry, authority: &Authority) -> Outcome
{
    
    match authority.resource_role()
    {
        Role::RoleGM(_, game_id) => {
            

            let Some(game) = registry.get_mut_game(game_id) 
            else {return Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::NoMatchingGame})};
            if let Err(err) = game.start_combat_rounds()
            {
                match err.kind
                {
                    crate::tracker::game::ErrorKind::InvalidStateAction => {
                        Outcome::Error(Error{ message: err.msg, kind: ErrorKind::InvalidStateAction })
                    },
                    _ => {unreachable!()}
                }
            }
            else 
            {
                Outcome::CombatRoundStarted
            }
        }
        _ => Outcome::Error(Error {message: String::from("Only the game's GM may initiate combat."), kind: ErrorKind::UnauthorizedAction})
    }
}


pub fn try_advance_turn2(registry: &mut GameRegistry, authority: &Authority) -> (Outcome, Option<Notification>)
{
    let response: Outcome;

    let (game, game_id) = match authority.resource_role() {
        Role::RoleGM(_, game_id) => {
            let Some(game) = registry.get_mut_game(game_id)
            else { return (Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::UnknownId}), None)};
            (game, game_id)
        }
        _ => return (Outcome::Error(Error { message: String::from("Only the game's GM may advance the turn."), kind: ErrorKind::UnauthorizedAction }), None)
    };

    match game.advance_round()
    {
        Ok(()) => {

            let senders = game.get_combatants().iter()
                            .map(|char_id| registry.players_by_character(game_id, char_id))
                            .filter(|player_id_opt| player_id_opt.is_some())
                            .map(|player_id_opt| player_id_opt.unwrap())
                            .map(|player_id| registry.get_player_sender(player_id))
                            .map(|player_sender_opt| player_sender_opt.unwrap())
                            .collect::<Vec<Sender<Arc<WhatChanged>>>>();
            (Outcome::TurnAdvanced, Some(Notification { change_type: Arc::from(WhatChanged::TurnAdvanced), send_to: senders }))
        }, 
        Err(GameError{msg, kind: crate::tracker::game::ErrorKind::InvalidStateAction}) => {
            (Outcome::Error(Error{message: msg, kind: ErrorKind::InvalidStateAction}), None)
        }, 
        Err(GameError{msg, kind: crate::tracker::game::ErrorKind::UnresolvedCombatant}) => {
            (Outcome::Error(Error{message: msg, kind: ErrorKind::CannotAdvanceTurn}), None)
        },
        Err(GameError{msg, kind: crate::tracker::game::ErrorKind::EndOfInitiative}) => {
            (Outcome::Error(Error{message: msg, kind: ErrorKind::NoEventsLeft}), None)
        },
        _ => unreachable!("The other game ErrorKind types should not exist.")
    }
}

pub fn try_advance_turn(registry: &mut GameRegistry, authority: &Authority) -> Outcome
{
    let response: Outcome;

    let game = match authority.resource_role() {
        Role::RoleGM(_, game_id) => {
            let Some(game) = registry.get_mut_game(game_id)
            else { return Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::UnknownId})};
            game
        }
        _ => return Outcome::Error(Error { message: String::from("Only the game's GM may advance the turn."), kind: ErrorKind::UnauthorizedAction })
    };

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
        response = Outcome::TurnAdvanced;
    }

    return response;
}


fn take_action2(registry: &mut GameRegistry, action: &Action, authority: &Authority) -> (Outcome, Option<Notification>)
{

    let (game, game_id, player_id) = match authority.resource_role() 
    {
        Role::RoleGM(player_id, game_id) | Role::RolePlayer(player_id, game_id) => {
            if registry.characters_by_player(game_id, player_id).map_or(false, |chars| chars.contains(&action.character_id))
            {
                let Some(game) = registry.get_mut_game(game_id)
                else {return (Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::NoMatchingGame}), None)};
                (game, game_id, player_id)
            }
            else {
                return (Outcome::Error(Error {message: String::from("Only the owner of a character may take an action for it."), kind: ErrorKind::UnauthorizedAction}), None);
            }
        }
        _ => return (Outcome::Error(Error{message: String::from("Unregistered or observing players have no character to act on."), kind: ErrorKind::UnauthorizedAction}), None)
    };

    match game.take_action(action.character_id, action.action)
    {
        Ok(_) => 
        {
            let notification = registry.gm_sender(game_id)
                .map(|sender| {
                    let mut senders = Vec::with_capacity(1);
                    senders.push(sender);
                    Notification { change_type: Arc::from(WhatChanged::PlayerActed), send_to:  senders}
                });
            (Outcome::ActionTaken, notification)
        },
        Err(err) => 
        {
            match err.kind
            {
                crate::tracker::game::ErrorKind::InvalidStateAction => 
                    {(Outcome::Error(Error{message: err.msg, kind: ErrorKind::InvalidStateAction}), None)},
                crate::tracker::game::ErrorKind::UnknownCastId => 
                    {(Outcome::Error(Error{message: err.msg, kind: ErrorKind::NoSuchCharacter}), None)},
                crate::tracker::game::ErrorKind::EndOfInitiative => 
                    {(Outcome::Error(Error{message:err.msg, kind: ErrorKind::CannotAdvanceTurn}), None)},
                crate::tracker::game::ErrorKind::NoAction => 
                    {(Outcome::Error(Error{message: err.msg, kind: ErrorKind::NoActionLeft}), None)},
                crate::tracker::game::ErrorKind::UnresolvedCombatant => 
                    {(Outcome::Error(Error{message: err.msg, kind: ErrorKind::NotCharactersTurn}), None)},
                _ => {unreachable!("Should not be called.")}
            }
        },
    }
}

fn take_action(registry: &mut GameRegistry, action: &Action, authority: &Authority) -> Outcome
{

    let (game, game_id, player_id) = match authority.resource_role() 
    {
        Role::RoleGM(player_id, game_id) | Role::RolePlayer(player_id, game_id) => {
            if registry.characters_by_player(game_id, player_id).map_or(false, |chars| chars.contains(&action.character_id))
            {
                let Some(game) = registry.get_mut_game(game_id)
                else {return Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::NoMatchingGame})};
                (game, game_id, player_id)
            }
            else {
                return Outcome::Error(Error {message: String::from("Only the owner of a character may take an action for it."), kind: ErrorKind::UnauthorizedAction});
            }
        }
        _ => return Outcome::Error(Error{message: String::from("Unregistered or observing players have no character to act on."), kind: ErrorKind::UnauthorizedAction})
    };

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

fn list_current_turn_events(game_registry: &mut GameRegistry, authority: &Authority) -> Outcome
{
    let game = match authority.resource_role() {
        Role::RoleGM(_, game_id) | Role::RolePlayer(_, game_id) | Role::RoleObserver(_, game_id) => 
        {
            let Some(game) = game_registry.get_game(game_id)
            else {return Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::NoMatchingGame}) };
            game
        },
        _ => {
            return Outcome::Error(Error {message: String::from("Only registered players and observers may view game events."), kind: ErrorKind::UnauthorizedAction});
        }
    };

    Outcome::MatchingEventsAre(game.currently_up())
}

fn list_unresolved_events(registry: &GameRegistry, authority: &Authority) -> Outcome
{
    let game = match authority.resource_role() {
        Role::RoleGM(_, game_id) | Role::RolePlayer(_, game_id) | Role::RoleObserver(_, game_id) => 
        {
            let Some(game) = registry.get_game(game_id)
            else {return Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::NoMatchingGame}) };
            game
        },
        _ => {
            return Outcome::Error(Error {message: String::from("Only registered players and observers may view game events."), kind: ErrorKind::UnauthorizedAction});
        }
    };

    Outcome::MatchingEventsAre(game.waiting_for())
}

fn list_next_turn_events(registry: &mut GameRegistry, authority: &Authority) -> Outcome
{
    match authority.resource_role() {
        Role::RoleGM(_, game_id) | Role::RolePlayer(_, game_id) | Role::RoleObserver(_, game_id) => 
        {
            let Some(game) = registry.get_game(game_id)
            else { return Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::NoMatchingGame}) };
            Outcome::MatchingEventsAre(game.on_deck())
        },
        _ => 
        {
            return Outcome::Error(Error {message: String::from("Only registered players and observers may view game events."), kind: ErrorKind::UnauthorizedAction});
        }
    }
    
}

fn list_all_events_by_id_this_pass(registry: &mut GameRegistry, authority: &Authority) -> Outcome
{
    match authority.resource_role() {
        Role::RoleGM(_, game_id) | Role::RolePlayer(_, game_id) | Role::RoleObserver(_, game_id) =>
        {
            let Some(game) = registry.get_game(game_id)
            else { return Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::NoMatchingGame}) };
            Outcome::MatchingEventsById(game.collect_all_remaining_events())
        }
        _ => 
        {
            return Outcome::Error(Error {message: String::from("Only registered players and observers may view game events."), kind: ErrorKind::UnauthorizedAction});
        }
    }
    
}

fn next_initiative(registry: &mut GameRegistry, authority: &Authority) -> Outcome
{

    match authority.resource_role() {
        Role::RoleGM(_, game_id) | Role::RolePlayer(_, game_id) | Role::RoleObserver(_, game_id) =>
        {
            let Some(game) = registry.get_game(game_id)
            else { return Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::NoMatchingGame}) };
            Outcome::InitiativeIs(game.get_next_init())
        }
        _ =>
        {
            return Outcome::Error(Error {message: String::from("Only registered players and observers may view game events."), kind: ErrorKind::UnauthorizedAction});
        }
    }
}

fn current_initiative(registry: &mut GameRegistry, authority: &Authority) -> Outcome
{
    match authority.resource_role() {
        Role::RoleGM(_, game_id) | Role::RolePlayer(_, game_id) | Role::RoleObserver(_, game_id) =>
        {
            let Some(game) = registry.get_game(game_id)
            else { return Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::NoMatchingGame}) };
            Outcome::InitiativeIs(game.get_current_init())
        }
        _ =>
        {
            return Outcome::Error(Error {message: String::from("Only registered players and observers may view game events."), kind: ErrorKind::UnauthorizedAction});
        }
    }
}

fn remaining_initiatives_are(registry: &mut GameRegistry, authority: &Authority) -> Outcome
{
    match authority.resource_role() {
        Role::RoleGM(_, game_id) | Role::RolePlayer(_, game_id) | Role::RoleObserver(_, game_id) =>
        {
            let Some(game) = registry.get_game(game_id)
            else { return Outcome::Error(Error {message: String::from("The game ID does not resolve to a running game."), kind: ErrorKind::NoMatchingGame}) };
            Outcome::InitiativesAre(game.get_all_remaining_initiatives())
        }
        _ =>
        {
            return Outcome::Error(Error {message: String::from("Only registered players and observers may view game events."), kind: ErrorKind::UnauthorizedAction});
        }
    }
    
}
