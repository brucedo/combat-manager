use rocket::serde::{Serialize, Deserialize};
use rocket::form::FromForm;
use uuid::Uuid;

use crate::tracker::character::Character;

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

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct GMView<'a>
{
    pub game_id: Uuid,
    pub pcs: Vec<SimpleCharacterView<'a>>,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SimpleCharacterView<'r>
{
    pub char_name: &'r str,
    pub char_id: &'r Uuid,
}

impl From<Character> for SimpleCharacterView<'_>
{
    fn from(_: Character) -> Self {
        todo!()
    }
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