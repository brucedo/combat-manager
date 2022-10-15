use std::collections::{HashMap, HashSet};

use super::gear::{Weapon, Armour};

pub struct Character
{
    pub name: String,
    // pub id: Uuid,
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
            // id: Uuid::new_v4(),
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
            // id: Uuid::new_v4(),
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

#[derive(Copy, Clone)]
pub enum Metatypes
{
    Human,
    Dwarf,
    Elf,
    Troll,
    Orc,
}

pub struct Quality
{
    pub name: String,
    pub stat_modifier: i8,
    pub skill_modifier: i8,
}

pub struct Skill
{
    pub name: String,
    pub subtype: Option<String>,
    pub stat: String,
    pub specialized: bool,
    pub specialization_type: String,
    pub rating: i8
}