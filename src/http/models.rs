use std::sync::Arc;

use uuid::Uuid;

use crate::tracker::character::{Character, Metatypes};

// #[derive(Serialize, Deserialize)]
pub struct IndexModel<'r>
{
    pub player_handle: &'r str,
    pub summaries: Vec<GameSummary>
}

// #[derive(Serialize, Deserialize)]
pub struct GameSummary
{
    pub game_name: String,
    pub url: String,
    pub gm: Uuid
}

// #[derive(Serialize)]
pub struct GMView
{
    pub game_id: Uuid,
    pub pcs: Vec<SimpleCharacterView>,
    pub npcs: Vec<SimpleCharacterView>,
}

// #[derive(Serialize)]
pub struct SimpleCharacterView
{
    pub char_name: String,
    pub char_id: Uuid,
    pub metatype: Metatypes,
}

impl From<Character> for SimpleCharacterView
{
    fn from(src: Character) -> Self {
        SimpleCharacterView { char_name: src.name.clone(), char_id: src.id.clone(), metatype: src.metatype }
    }
}

impl From<&Character> for SimpleCharacterView
{
    fn from(src: &Character) -> Self {
        SimpleCharacterView { char_name: src.name.clone(), char_id: src.id.clone(), metatype: src.metatype }
    }
}

// #[derive(Serialize)]
pub struct PlayerView
{
    pub player_handle: Arc<String>,
    pub game_id: Uuid,
    pub game_name: String,
    pub character_state: Option<SimpleCharacterView>
}

// #[derive(Serialize)]
// #[serde(crate = "rocket::serde")]
// pub enum CharacterState 
// {
//     Generated(SimpleCharacterView),
//     NotGenerated
// }


// #[derive(FromForm)]
pub struct NewGame<'r>
{
    pub game_name: &'r str
}

// #[derive(FromForm)]
pub struct NewCharacter<'r>
{
    pub char_name: &'r str,
    pub metatype: &'r str,
    pub is_npc: bool,
}

impl From<NewCharacter<'_>> for Character
{
    fn from(npc: NewCharacter<'_>) -> Self {
        let metatype: Metatypes;
        match npc.metatype
        {
            "Human" => {metatype = Metatypes::Human},
            "Dwarf" => {metatype = Metatypes::Dwarf},
            "Elf" => {metatype = Metatypes::Elf},
            "Orc" => {metatype = Metatypes::Orc},
            "Troll" => {metatype = Metatypes::Troll},
            _ => {metatype = Metatypes::Human}
        }

        let mut char = Character::new_npc(metatype, String::from(npc.char_name));
        char.player_character = !npc.is_npc;
        return char;
    }
}