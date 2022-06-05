use std::collections::HashMap;

use log::debug;
use uuid::Uuid;

use super::{character::Character};


pub struct Game {
    current_state: State,

    cast: HashMap<Uuid, Character>,

    // Combat data
    
    initiatives: Vec<i8>,
    next_init_index: usize,
    current_initiative: i8,
    initiative_pass: usize,
    initiative_player_map: HashMap<i8, Vec<Uuid>>,
    combatant_data: HashMap<Uuid, CharacterCombatData>,
    participating: Vec<Uuid>
    
}



impl Game {
    pub fn new() -> Game
    {
        Game {
            current_state: State::PreCombat,
            cast: HashMap::new(),

            // Combat specific data
            initiatives: Vec::new(),
            next_init_index: 0,
            current_initiative: -1,
            initiative_pass: 0,
            initiative_player_map: HashMap::new(),
            participating: Vec::new(),
            combatant_data: HashMap::new()
        }
    }

    // **********************************************************************************
    // Game specific setup and upkeep

    pub fn add_cast_member(self: &mut Game, cast_member: Character)
    {
        self.cast.insert(cast_member.id, cast_member);
    }

    pub fn cast_size(self: &mut Game) -> usize
    {
        self.cast.len()
    }

    pub fn retire_cast_member(self: &mut Game, cast_member_id: Uuid)
    {
        self.cast.remove(&cast_member_id);
    }

    // **********************************************************************************
    // State retrieval methods

    pub fn current_state(self: &mut Game)->String
    {
        self.current_state.to_string()
    }

    pub fn waiting_for(self: &mut Game)->Option<Vec<Uuid>>
    {
        if self.current_state != State::InitiativePass
        {
            return Option::None;
            // return Err(GameError::new
            // (
            //     ErrorKind::InvalidStateAction, 
            //     String::from("Not in an initiative pass phase - cannot identify who should be acting."
            // )));
        }

        if let Some(initiative) = self.initiatives.get(self.next_init_index)
        {
            if let Some(char_id) = self.initiative_player_map.get(initiative)
            {
                return Some((*char_id).clone());
            }
            // else
            // {
            //     return Err(GameError::new
            //     (
            //         ErrorKind::GameStateInconsistency,
            //         String::from("There is an initiative recorded for no matching character ID.  The game is in an inconsistent state.")
            //     ));
            // }
        }
        // else {
        //     return Err(GameError::new
        //     (
        //         ErrorKind::GameStateInconsistency, 
        //         String::from("The Game object has entered into an inconsistent state.  Clearly the developer sucks.  You should go complain at them.")
        //     ));
        // }
        return None;

    }

    pub fn on_deck(self: &Game) -> Option<Vec<Uuid>>
    {
        if self.current_state != State::InitiativePass
        {
            return None;
            // return Err(GameError::new
            // (
            //     ErrorKind::InvalidStateAction, 
            //     String::from("Not in an initiative pass phase - cannot identify who should be acting next."
            // )));
        }

        let next = self.find_next_participant();
        if next >= self.initiatives.len()
        {
            return None;
            // return Err(GameError::new
            // (
            //     ErrorKind::EndOfInitiative,
            //     String::from("No further players will be acting until the next Combat Round.")
            // ));
        }

        if let Some(initiative) = self.initiatives.get(next)
        {
            if let Some(char_id) = self.initiative_player_map.get(initiative)
            {
                return Some((*char_id).clone());
            }
            // else {
            //     return Err(GameError::new
            //     (
            //         ErrorKind::GameStateInconsistency,
            //         String::from("The Game object has an initiative that does not map to a character.")
            //     ));
            // }
        }
        // else
        // {
        //     return Err(GameError::new
        //     (
        //         ErrorKind::GameStateInconsistency,
        //         String::from("The Game object found an initiative index but now cannot find the initiative.  This probably should not happen.")
        //     ));
        // }
        return None;
    }

    pub fn get_combatants(self: &Game) -> Vec<Uuid>
    {
        self.participating.clone()
    }


    // ******************************************************************************************
    // State change methods

    pub fn end_combat(self: &mut Game)
    {
        self.current_state = State::PreCombat;
        self.initiative_player_map.clear();
        self.participating.clear();
    }

    pub fn add_combatant(self: &mut Game, combatant: Uuid) -> Result<(), GameError>
    {
        if !self.cast.contains_key(&combatant)
        {
            return Err(GameError::new
            (
                ErrorKind::UnknownCastId, String::from(format!("ID {} does not match against any ID in the cast list.", combatant))
            ));
        }
        let mut combatant_data = CharacterCombatData::new(combatant);

        // TODO: Look up character and review their gear, augs etc. to fill in turns_per_round and/or update any other fields
        self.fill_combatant_data(combatant, &mut combatant_data);
        self.combatant_data.insert(combatant, combatant_data);

        Ok(())
    }

    fn fill_combatant_data(self: &mut Game, combatant: Uuid, data: &mut CharacterCombatData)
    {
        data.actions.insert(ActionType::FREE, 1);
        data.actions.insert(ActionType::COMPLEX, 1);
        data.actions.insert(ActionType::SIMPLE, 2);
    }

    pub fn add_combatants(self: &mut Game, mut involved: Vec<Uuid>) -> Result<(), GameError>
    {
        if self.current_state != State::PreCombat
        {
            return Err(GameError{
                kind: ErrorKind::InvalidStateAction,
                msg: String::from("Cannot begin combat from any state other than PreCombat.")
            });
        }

        let mut bad_ids = Vec::<String>::new();

        // Set up Characters;
        for id in involved.drain(0..involved.len() - 1)
        {
            if self.cast.contains_key(&id)
            {
                let mut combatant_data = CharacterCombatData::new(id);
                self.fill_combatant_data(id, &mut combatant_data);
                self.combatant_data.insert(id, combatant_data);
            }
            else
            {
                bad_ids.push(id.to_string());
            }
        }

        if bad_ids.len() > 0 {
            let missing_ids = bad_ids.join(", ");
            return Err(GameError{
                kind: ErrorKind::UnknownCastId,
                msg: String::from(format!("The character(s) with id(s) {} is not registered as a cast member of this adventure.", missing_ids))
            });
        }

        Ok(())
    }

    pub fn begin_initiative(self: &mut Game) -> Result<(), GameError>
    {
        debug!("Starting initiative.");
        if self.current_state != State::PreCombat && self.current_state != State::InitiativePass
        {
            debug!("Current state of game {} is not allowed to transition into Initiative.", self.current_state.to_string());
            return Err(GameError::new
            (
                ErrorKind::InvalidStateAction, String::from("You may not call begin_initiative unless in the PreCombat or InitiativePass phase.")
            ));
        }

        if self.combatant_data.len() == 0
        {
            debug!("The play field has not had any combatants identified.");
            return Err(GameError::new
            (
                ErrorKind::NoCombatants, String::from("You may not begin an initiative round if no one is going to fight.")
            ))
        }
        
        self.current_state = State::Initiative;

        Ok(())
    }

    pub fn add_initiative(self: &mut Game, character: Uuid, initiative: i8) -> Result<(), GameError>
    {
        if self.current_state != State::Initiative
        {
            return Err(GameError{
                kind: ErrorKind::InvalidStateAction,
                msg: String::from(format!("The game is not in the initiative phase: you cannot add a new initiative roll."))
            });
        }

        if self.combatant_data.contains_key(&character)
        {
            self.initiatives.push(initiative);
            let characters = self.initiative_player_map.entry(initiative).or_insert(Vec::new());
            characters.push(character);
        }
        else
        {
            return Err(GameError{
                kind: ErrorKind::UnknownCastId,
                msg: format!("The character referenced by UUID {} does not exist.", character)
            });
        }

        Ok(())
    }

    pub fn begin_initiative_passes(self: &mut Game) -> Result<(), GameError>
    {
        if self.initiatives.len() != self.combatant_data.len()
        {
            return Err(GameError::new(
                ErrorKind::InvalidStateAction, 
                String::from("Not all combatants have supplied their initiative.  Cannot begin passes.")
            ));
        }

        self.initiatives.sort_by(|a, b| b.cmp(a));
        self.initiative_pass = 1;
        self.next_init_index = 0;
        self.current_initiative = *self.initiatives.get(0).unwrap();
        self.current_state = State::InitiativePass;

        Ok(())
    }

    pub fn advance_initiative_pass(self: &mut Game) -> Result<(), GameError>
    {
        if self.current_state != State::InitiativePass
        {
            return Err(GameError{
                kind: ErrorKind::InvalidStateAction,
                msg: String::from(format!("The game is not in the character turn phase.  You cannot begin an initiative turn."))
            })
        }

        // start at the zero mark, and then do a quick scan to find the first character who has a turn this initiative round.
        // (Remember: this will always be zero on the first round but may NOT be zero on subsequent ones.)
        self.next_init_index = 0;
        self.initiative_pass += 1;

        // If the first initiative entry cannot participate on this initiative pass
        if !self.participate_in_pass(self.next_init_index)
        {
            // Find the next that can.
            self.next_init_index = self.find_next_participant();
        }

        // If the value of self.next_init_index after these two checks is actually > the number of characters participating in combat,
        // then there are no events that can occur in this round, and the initiative round is over.
        if self.next_init_index >= self.initiatives.len(){
            self.current_initiative = -1;
            return Err(GameError::new(
                ErrorKind::EndOfInitiativePass,
                String::from("All characters that can act in this pass have acted.")
            ));
        }
        else {
            self.current_initiative = *(self.initiatives.get(self.next_init_index).unwrap());
        }

        Ok(())
    }

    fn participate_in_pass(self: &Game, index: usize) -> bool
    {
        let initiative = self.initiatives.get(index).unwrap();
        if let Some(char_ids) = self.initiative_player_map.get(&initiative)
        {
            for char_id in char_ids 
            {
                if let Some(character_data) = self.combatant_data.get(&char_id)
                {
                    if character_data.initiative_passes >= self.initiative_pass {
                        return true;
                    }
                }
            }
        }

        return false;
    }

    fn find_next_participant(self: &Game) -> usize
    {
        let mut start = self.next_init_index + 1;
        while start < self.initiatives.len()
        {
            if self.participate_in_pass(start)
            {
                return start;
            }
            start += 1;
        }

        return start;
    }

    pub fn advance_initiative(self: &mut Game) -> Result<(), GameError>
    {
        if self.current_state != State::InitiativePass
        {
            return Err(GameError{
                kind: ErrorKind::InvalidStateAction,
                msg: String::from(format!("The game is not in the character turn phase.  You cannot advance the action in this way."))
            })
        }

        // Make sure all current characters have signalled they are done
        if self.current_initiative >= 0
        {
            for character in self.initiative_player_map.get(&self.current_initiative).unwrap()
            {
                if let Some(char_data) = self.combatant_data.get(character)
                {
                    if !char_data.has_resolved
                    {
                        return Err(GameError::new(
                            ErrorKind::UnresolvedCombatant,
                            String::from(format!("Character ID {} has not yet signalled their turn is complete.", character))
                        ));
                    }
                }
            }
        }
        else {
            return Err(GameError::new(
                ErrorKind::UnknownCastId,
                String::from("Initiative turn appears to have exhausted all eligible characters.")
            ));
        }
        

        self.next_init_index = self.find_next_participant();

        if self.next_init_index >= self.initiatives.len()
        {
            return Err(GameError::new(
                ErrorKind::EndOfInitiative,
                String::from("The end of initiative order has been reached for this pass.  You must advance to the next pass to continue combat.")
            ));
        }

        Ok(())
    }

    pub fn take_action(self: &mut Game, actor: Uuid, action_type: ActionType) -> Result<(), GameError>
    {

        if self.current_state != State::InitiativePass
        {
            return Err(GameError::new(ErrorKind::InvalidStateAction, String::from(format!("The game is not in the character turn phase.  You cannot take an action."))));
        }

        // Rules for taking action: 
        // If it is the current initiative of the actor trying to act, then the actor may attempt to perform any of their actions.
        // if it is NOT the current initiative of the actor trying to act, they may only take free actions.

        // So - get the actors for the current initiative out
        let result = self.initiative_player_map.get_mut(&self.current_initiative);
        if result.is_none()
        {
            return Err(GameError::new(
                ErrorKind::EndOfInitiative,
                String::from(format!("The current initiative value {} does not map to any valid combatants.", self.current_initiative))
            ))
        }

        let current_combatants = result.unwrap();
        

        if current_combatants.contains(&actor) || action_type == ActionType::FREE
        {
            match self.combatant_data.entry(actor)
            {
                std::collections::hash_map::Entry::Occupied(mut entry) => 
                {
                    let combat_data = entry.get_mut();
                    let actions = &mut combat_data.actions;
                    // unwrapping should be safe so long as the methods maintain the presence of all action types as an invariant.
                    let mut action_count = actions.remove(&action_type).unwrap();
                    if action_count > 0 { action_count -= 1;}
                    actions.insert(action_type, action_count);
                },
                std::collections::hash_map::Entry::Vacant(_) => 
                {
                    return Err(GameError::new(
                        ErrorKind::UnknownCastId,
                        String::from(format!("The combat data for combatant {} was not recorded.", actor))
                    ));
                },
            }
        }
        

        
        Ok(())
    }



}

pub struct CharacterCombatData {
    id: Uuid,
    initiative_passes: usize,
    actions: HashMap<ActionType, usize>,
    free_actions: usize,
    simple_actions: usize,
    complex_actions: usize,
    has_resolved: bool,

}

impl CharacterCombatData {
    pub fn new(id: Uuid)->CharacterCombatData {
        CharacterCombatData 
        { 
            id, 
            initiative_passes: 0, 
            free_actions: 1, 
            simple_actions: 2, 
            complex_actions: 1, 
            actions: HashMap::new(),
            has_resolved: false,
        }
    }

    pub fn reset(self: &mut CharacterCombatData) {
        self.free_actions = 1;
        self.simple_actions = 2;
        self.complex_actions = 1;
        self.has_resolved = false;
    }

    pub fn resolve(self: &mut CharacterCombatData) {
        self.has_resolved = true;
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
enum State {
    PreCombat,
    Initiative,
    InitiativePass,
    PostRound,
    Other,
}

impl State {
    pub fn to_string(self: &State) -> String
    {
        match self {
            State::PreCombat => String::from("PreCombat"),
            State::Initiative => String::from("Initiative Rolls"),
            State::InitiativePass => String::from("Initiative Pass"),
            State::PostRound => String::from("End Of Round"),
            State::Other => String::from("Other"),
        }
    }
}

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum ActionType {
    FREE,
    SIMPLE,
    COMPLEX
}

#[derive(Debug)]
pub struct GameError {
    kind: ErrorKind,
    msg: String,
}

impl GameError {
    pub fn new(kind: ErrorKind, msg: String) -> GameError
    {
        GameError{kind, msg}
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    InvalidStateAction,
    UnknownCastId,
    EndOfInitiative,
    EndOfInitiativePass,
    NoAction,
    GameStateInconsistency,
    UnresolvedCombatant,
    NoCombatants
}

#[derive(Debug)]
pub enum GameValue {
    PlayerId(Uuid),
    CurrentState(String),
}


#[cfg(test)]
mod tests
{
    use uuid::Uuid;

    use crate::game::{game::{GameValue, State}, character::{Character, Metatypes}};

    use super::Game;

    pub fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    macro_rules! populate {
        ($game:expr, $char:expr) => {
            let local_game: &mut Game = $game;
            let char_id:Uuid = $char.id;
            local_game.add_cast_member($char);
            local_game.add_combatant(char_id);
        };
        ($game:expr, $char:expr, $($chars:expr),+) => {
            let local_game: &mut Game = $game;
            let char_id:Uuid = $char.id;
            local_game.add_cast_member($char);
            local_game.add_combatant(char_id);
            populate!(local_game, $($chars),+);
            
        };
    }


    fn build_orc()->Character
    {
        Character::new_pc(Metatypes::Orc, String::from("Zorc"))
    }

    fn build_elf()->Character
    {
        Character::new_pc(Metatypes::Elf, String::from("Lef"))
    }

    fn build_dwarf()->Character
    {
        Character::new_npc(Metatypes::Dwarf, String::from("Dorf"))
    }


    #[test]
    pub fn build_game()
    {
        let mut game = Game::new();

        assert_eq!(game.current_state(), String::from("PreCombat"));
        assert_eq!(game.waiting_for(), None);
        assert_eq!(game.on_deck(), None);
        assert_eq!(game.get_combatants().len(), 0);
    }

    #[test]
    pub fn test_adding_cast_member()
    {
        let cast_member = Character::new_pc(Metatypes::Human, String::from("Demo"));
        let mut game: Game = Game::new();

        game.add_cast_member(cast_member);

        assert_eq!(game.cast_size(), 1);
        
    }

    #[test]
    pub fn test_removing_cast_member()
    {
        let cast_member = Character::new_pc(Metatypes::Elf, String::from("Delfmo"));
        let id = cast_member.id;
        let mut game: Game = Game::new();

        game.add_cast_member(cast_member);
        game.retire_cast_member(id);
        assert_eq!(game.cast_size(), 0);
    }

    #[test]
    pub fn test_adding_combatant_not_in_cast()
    {
        let mut game = Game::new();

        let combatant_id = Uuid::new_v4();

        let result = game.add_combatant(combatant_id);

        assert!(result.is_err());
    }

    #[test]
    pub fn test_adding_real_combatant()
    {
        let dorf = Character::new_npc(Metatypes::Dwarf, String::from("Dorf"));
        let torll = Character::new_npc(Metatypes::Troll, String::from("Torll"));
        let mut game = Game::new();

        let combatant_id = dorf.id;

        game.add_cast_member(dorf);
        game.add_cast_member(torll);

        let result = game.add_combatant(combatant_id);

        assert!(result.is_ok());
    }

    #[test]
    pub fn test_initative_transition()
    {
        let lef = Character::new_npc(Metatypes::Elf, String::from("Lef"));
        let lef_id = lef.id;
        let zorc = Character::new_npc(Metatypes::Orc, String::from("Zorc"));
        let zorc_id = zorc.id;
        let mut game = Game::new();

        game.add_cast_member(lef);
        game.add_cast_member(zorc);

        game.add_combatant(lef_id);
        game.add_combatant(zorc_id);

        let result = game.begin_initiative();

        assert!(result.is_ok());
        assert_eq!(game.current_state(), String::from("Initiative Rolls"));
    }

    #[test]
    pub fn test_initiative_transition_fail()
    {
        let lef = Character::new_npc(Metatypes::Elf, String::from("Lef"));
        let lef_id = lef.id;

        let mut game = Game::new();

        game.add_cast_member(lef);

        let result = game.begin_initiative();

        assert!(result.is_err());
        assert_eq!(game.current_state(), String::from("PreCombat"));
    }

    #[test]
    pub fn add_initiative_and_fail()
    {
        let lef = Character::new_npc(Metatypes::Elf, String::from("Lef"));
        let lef_id = lef.id;
        let mut game = Game::new();

        game.add_cast_member(lef);
        game.add_combatant(lef_id);

        let result = game.add_initiative(lef_id, 4);

        assert!(result.is_err());
    }

    #[test]
    pub fn add_initative_and_succeed()
    {
        init();

        let zorc = build_orc();
        let zorc_id = zorc.id;
        let dorf = build_dwarf();
        let dorf_id = dorf.id;

        let mut game = Game::new();
        populate!(&mut game, zorc, dorf);
        game.begin_initiative();

        let result = game.add_initiative(zorc_id, 4);

        assert!(result.is_ok());
    }

    #[test]
    pub fn add_two_initatives_and_succeed()
    {
        init();


        let zorc = build_orc();
        let zorc_id = zorc.id;
        let dorf = build_dwarf();
        let dorf_id = dorf.id;

        

        let mut game = Game::new();
        populate!(&mut game, zorc, dorf);
        game.begin_initiative();

        let result = game.add_initiative(zorc_id, 2);
        assert!(result.is_ok());
        let result = game.add_initiative(dorf_id, 13);
        assert!(result.is_ok());
    }

    #[test]
    pub fn add_init_for_no_character()
    {
        init();

        let zorc = build_orc();
        let mork = build_orc();

        let mut game = Game::new();
        populate!(&mut game, zorc, mork);
        game.begin_initiative();

        let result = game.add_initiative(Uuid::new_v4(), 2);
        assert!(result.is_err());
    }



    #[test]
    pub fn advance_initiative()
    {
        init();

        let zorc = build_orc();
        let zorc_id = zorc.id;
        let mork = build_orc();
        let mork_id = mork.id;
        let dork = build_dwarf();
        let dork_id = dork.id;

        let mut game = Game::new();
        populate!(&mut game, zorc, mork, dork);

        game.begin_initiative();

        game.add_initiative(zorc_id, 4);
        game.add_initiative(mork_id, 8);
        game.add_initiative(dork_id, 3);

        let result = game.begin_initiative_passes();

        assert_eq!(game.current_state(), String::from("Initiative Pass"));
        assert!(result.is_ok());

    }

    #[test]
    pub fn advance_incomplete_initiative()
    {
        init();

        let zorc = build_orc();
        let zorc_id = zorc.id;
        let melf = build_elf();
        let dork = build_dwarf();
        let dork_id = dork.id;

        let mut game = Game::new ();
        populate!(&mut game, zorc, dork, melf);

        game.begin_initiative();
        game.add_initiative(zorc_id, 1);
        game.add_initiative(dork_id, 15);

        let result = game.begin_initiative_passes();

        assert_eq!(game.current_state(), String::from("Initiative Rolls"));
        assert!(result.is_err());
    }


}