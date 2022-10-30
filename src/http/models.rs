use rocket::serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct IndexModel<'r>
{
    pub player_handle: &'r str,
    pub summaries: Vec<GameSummary>
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct GameSummary
{
    pub game_name: String,
    pub game_id: Uuid,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct GameSummaries
{
    pub games: Vec<GameSummary>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct GMView
{
    pub game_id: Uuid,
}
