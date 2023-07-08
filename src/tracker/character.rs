use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::gear::{Weapon, Armour};

pub struct Character
{
    pub name: String,
    pub id: Uuid,
    pub player_character: bool,
    pub metatype: Metatypes,
    pub stats: HashMap<String, i8>,
    pub qualities: HashSet<Quality>,
    pub skills: Vec<Skill>,
    pub weapons: Vec<Weapon>,
    pub armor: Vec<Armour>,
    pub physical_track_max: i8, // Total player health
    pub physical_track_filled: i8, // current damage
    pub stun_track_max: i8,
    pub stun_track_filled: i8,
    pub current_weapon_index: usize,
}

impl Character 
{
    pub fn new_pc(metatype: Metatypes, name: String) -> Character
    {
        Character {
            name,
            id: Uuid::new_v4(),
            player_character: true,
            metatype,
            stats: HashMap::new(),
            qualities: HashSet::new(),
            skills: Vec::new(),
            weapons: Vec::new(),
            armor: Vec::new(),
            physical_track_max: 0,
            physical_track_filled: 0,
            stun_track_max: 0,
            stun_track_filled: 0,
            current_weapon_index: 0,
        }
    }

    pub fn new_npc(metatype: Metatypes, name: String) -> Character
    {
        Character {
            name,
            id: Uuid::new_v4(),
            player_character: false,
            metatype,
            stats: HashMap::new(),
            qualities: HashSet::new(),
            skills: Vec::new(),
            weapons: Vec::new(),
            armor: Vec::new(),
            physical_track_max: 0,
            physical_track_filled: 0,
            stun_track_max: 0,
            stun_track_filled: 0,
            current_weapon_index: 0,
        }
    }
}

impl Clone for Character
{
    fn clone(&self) -> Self {    
        Self { 
            name: self.name.clone(), 
            id: self.id.clone(), 
            player_character: self.player_character.clone(), 
            metatype: self.metatype.clone(), 
            stats: self.stats.clone(), 
            qualities: self.qualities.clone(), 
            skills: self.skills.clone(), 
            weapons: self.weapons.clone(), 
            armor: self.armor.clone(), 
            physical_track_max: self.physical_track_max.clone(), 
            physical_track_filled: 
            self.physical_track_filled.clone(), 
            stun_track_max: self.stun_track_max.clone(), 
            stun_track_filled: self.stun_track_filled.clone(), 
            current_weapon_index: self.current_weapon_index.clone() 
        }
    }
}

// #[derive(Copy, Clone, Serialize, Deserialize)]
#[derive(Copy, Clone)]
pub enum Metatypes
{
    Human,
    Dwarf,
    Elf,
    Troll,
    Orc,
}

#[derive(Clone)]
pub struct Quality
{
    pub name: String,
    pub stat_modifier: i8,
    pub skill_modifier: i8,
}

#[derive(Clone)]
pub struct Skill
{
    pub name: String,
    pub subtype: Option<String>,
    pub stat: String,
    pub specialized: bool,
    pub specialization_type: String,
    pub rating: i8
}