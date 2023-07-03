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