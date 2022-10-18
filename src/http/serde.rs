use rocket::serde::{Serialize, Deserialize};
use uuid::Uuid;


#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct GameSummary
{
    pub game_name: String,
    pub game_id: Uuid,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct NewGameJson
{
    pub game_id: Uuid,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Character<'r>
{
    pub pc: bool,
    pub metatype: Metatypes,
    pub name: &'r str,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct AddedCharacterJson
{
    pub game_id: Uuid,
    pub char_id: Uuid,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct BeginCombat
{
    pub participants: Vec<Uuid>
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct NewState
{
    pub to_state: State,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub enum State
{
    Combat(BeginCombat),
    InitiativeRolls,
    InitiativePass,
    EndOfTurn,
}


#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct InitiativeRoll
{
    pub char_id: Uuid,
    pub roll: i8,
}


#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub enum Metatypes
{
    Human,
    Dwarf,
    Elf,
    Troll,
    Orc,
}