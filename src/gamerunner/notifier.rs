use std::sync::Arc;

use tokio::sync::mpsc::Sender as MpscSender;
use crate::tracker::character::Metatypes;

use super::{PlayerId, CharacterId, registry::GameRegistry, dispatcher::Outcome, GameId, authority::{self, Authority}};

pub struct Notification
{
    pub change_type: Arc<WhatChanged>, 
    pub send_to: Vec<MpscSender<Arc<WhatChanged>>>,
}

// #[derive(Clone)]
pub enum WhatChanged
{
    NewPlayer(PlayerJoined),
    NewCharacter(NewCharacter),
    StartingInitiativePhase,
    StartingCombatRound,
    PlayerActed,
    TurnAdvanced,
    PassAdvanced,
    RoundAdvanced,
    CombatStarted,
    UpNext,
    YourTurn,
    CombatEnded,
    GameEnded,
}

pub struct PlayerJoined
{
    pub name: String,
    pub player_id: PlayerId,
}

pub struct NewCharacter
{
    pub player_id: PlayerId,
    pub character_id: CharacterId,
    pub metatype: Metatypes,

}

// pub async fn notify_players(notification: WhatChanged, notification_list: &Vec<MpscSender<WhatChanged>>)
// {
    
//     for sender in notification_list
//     {
//         sender.send(notification.clone()).await;
//     }
// }

// pub fn into_notification(game_directory: &GameRegistry, outcome: &Outcome, authority: &Authority) -> Option<WhatChanged>
// {

    
//     match outcome {
//         Outcome::NewPlayer(_) => None,
//         Outcome::JoinedGame(player_info) => 
//         {
//             let player_name: &str = game_directory.player_name(&player_info.for_player)?;
//             Some(WhatChanged::NewPlayer(Arc::new(PlayerJoined{ name: String::from(player_name), player_id: player_info.for_player })))
//         },
//         Outcome::Destroyed => Some(WhatChanged::GameEnded),
//         Outcome::CharacterAdded((game_id, character_id)) => 
//         {
//             let game = game_directory.get_game(game_id)?;
//             let character = game.get_cast_by_id(character_id)?;
//             Some(WhatChanged::NewCharacter(Arc::new(NewCharacter { player_id: todo!(), character_id: *character_id, metatype: todo!() })))
            
//         },
//         Outcome::CombatStarted => Some(WhatChanged::CombatStarted),
//         Outcome::InitiativePhaseStarted => Some(WhatChanged::StartingInitiativePhase),
//         Outcome::InitiativeRollAdded => None,
//         Outcome::InitiativeStatus(_) => None,
//         Outcome::CombatRoundStarted => Some(WhatChanged::StartingCombatRound),
//         Outcome::ActionTaken => Some(WhatChanged::PlayerActed),
//         Outcome::TurnAdvanced => Some(WhatChanged::TurnAdvanced),
//         Outcome::CombatEnded => Some(WhatChanged::CombatEnded),
//         _ => None
//     }
// }