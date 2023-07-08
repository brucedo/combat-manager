
use uuid::Uuid;


// #[derive(Serialize, Deserialize)]
pub struct NewGame
{
    pub game_id: Option<Uuid>,
    pub game_name: String,
    pub gm_id: Option<Uuid>,
    pub gm_name: String,
}

// #[derive(Serialize, Deserialize)]
pub struct Character<'r>
{
    pub pc: bool,
    pub metatype: Metatypes,
    pub name: &'r str,
}

// #[derive(Serialize, Deserialize)]
pub struct AddedCharacterJson
{
    pub game_id: Uuid,
    pub char_id: Uuid,
}

// #[derive(Serialize, Deserialize)]
pub struct BeginCombat
{
    pub participants: Vec<Uuid>
}

// #[derive(Serialize, Deserialize)]
pub struct NewState
{
    pub to_state: State,
}

// #[derive(Serialize, Deserialize)]
pub enum State
{
    Combat(BeginCombat),
    InitiativeRolls,
    InitiativePass,
    EndOfTurn,
}


// #[derive(Serialize, Deserialize)]
pub struct InitiativeRoll
{
    pub char_id: Uuid,
    pub roll: i8,
}


// #[derive(Serialize, Deserialize)]
pub enum Metatypes
{
    Human,
    Dwarf,
    Elf,
    Troll,
    Orc,
}