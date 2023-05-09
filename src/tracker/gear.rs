#[derive(Clone)]
pub enum DamageType {
    Physical,
    Stun
}

pub enum ArmorTestType {
    Ballistic,
    Impact
}

#[derive(Clone)]
pub enum ReloadMethod {
    Clip,
    Break,
    Magazine,
    MuzzleLoader,
    Cylinder,
    Belt,
    Drum,
    SingleShot
}

#[derive(Clone)]
pub struct Weapon {
    pub weapon_type: String,
    pub weapon_name: String,
    pub assoc_skill: String,
    pub firing_features: Vec<FiringFeature>,
    pub reach: Option<i8>,
    pub electric: bool,
}

#[derive(Clone)]
pub struct FiringFeature {
    pub feature_name: String,
    pub reloads: ReloadMethod,
    pub reload_size: i8,
    pub armor_pen: i8,
    pub damage_type: DamageType,
    pub damage_equation: String, // this'll become a calculation later
    pub requires_reconfig: bool, // Requires an out-of-combat weapon reconfiguration to use this (see HK XM30)
    pub fire_modes: Vec<String>,
    pub recoil_comp: i8, // Some weapons have recoil compensation in the base config
    pub alt_recoil_comp: i8, // Some weapons can be reconfigured for better recoil compensation.
    pub current_fire_mode: usize,
}

pub struct AmmoTypes
{
    pub name: String,
    pub damage_modifier: Option<i8>,
    pub ap_modifier: Option<i8>,
    pub armor_test: ArmorTestType,
    pub electrical: bool
}

#[derive(Clone)]
pub struct Armour {
    pub name: String,
    pub ballistic_rating: i8,
    pub impact_rating: i8,
}