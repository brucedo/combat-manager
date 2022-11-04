use rocket::serde::{Serialize, Deserialize};
use rocket::form::FromForm;
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
    pub url: String,
    pub gm: Uuid
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct GMView
{
    pub game_id: Uuid,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct PlayerView
{
    pub game_id: Uuid,
    pub game_name: String,
}


#[derive(FromForm)]
pub struct NewGame<'r>
{
    pub game_name: &'r str
}