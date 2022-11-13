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
pub struct GMView
{
    pub game_id: Uuid,
    pub pcs: Vec<SimpleCharacterView>,
    pub npcs: Vec<SimpleCharacterView>,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SimpleCharacterView
{
    pub char_name: String,
    pub char_id: Uuid,
}

impl From<Character> for SimpleCharacterView
{
    fn from(src: Character) -> Self {
        SimpleCharacterView { char_name: src.name.clone(), char_id: src.id.clone() }
    }
}

impl From<&Character> for SimpleCharacterView
{
    fn from(src: &Character) -> Self {
        SimpleCharacterView { char_name: src.name.clone(), char_id: src.id.clone() }
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