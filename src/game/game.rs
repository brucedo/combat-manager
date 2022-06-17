use std::collections::{HashMap, hash_map::Entry};

use log::debug;
use uuid::Uuid;

use super::{character::Character, initiative::{InitTracker, PassState}};

// The game struct and methods coordinate actions and activity through combat.  The game struct is responsible for ensuring that
// initiative passes flow smoothly (albeit through the tracker), for keeping straight what actions a character can perform on any 
// given turn, and (eventually) what actual specific things they can do given their current state and inventory.

// Example:  The game object should be able to tell a character that, on their turn, they have 2 simple actions, one complex action,
// and a free action, or that they have a free action when it is not their turn.  If the player takes one simple action, that should 
// get marked down; if they take one complex action, then all of their simple actions and complex actions disappear for the rest of the pass.
//
// Example:  A character has a pistol readied.  The game object should be able to look at the character sheet, identify that the pistol is readied,
// and add appropriate pistol-actions that can be taken with either the simple or complex action.
//
// Example:  The character has just fired three rounds from a rifle.  Their rifle's ammunition counter should decrease by 3; if they attempt
// to fire another 3 round burst and only have one round left in the gun, the reduced damage rules should apply to that burst.

// State flow:  Begin Combat -> Load combatants -> Begin Combat Round -> Read Initiatives -> Begin Pass -> Check for more passes - End Combat Round
//                                                        ^                                       ^_____________________|                 |
//                                                        |_______________________________________________________________________________|
// Technically end combat would have a check for more rounds step.  But this is the basic flow - start a combat, load the combatants into 
// context (during which things like weapon state, skills, abilities etc. are scanned for modifiers), and then start passing through combat
// rounds.  Initiative tracking is now handled by the InitiativeTracker, so Game merely needs to call next() until the return type indicates
// we've hit the end.

pub struct Game {
    current_state: State,

    cast: HashMap<Uuid, Character>,

    // Combat data
    
    init_tracker: InitTracker,
    current_turn_id: Vec<Uuid>,
    next_id: Vec<Uuid>,
    current_initiative: i8,
    next_initiative: i8,
    // initiative_player_map: HashMap<i8, Vec<Uuid>>,
    combatant_data: HashMap<Uuid, CharacterCombatData>,
    
}



impl Game {
    pub fn new() -> Game
    {
        Game {
            current_state: State::PreCombat,
            cast: HashMap::new(),

            // Combat specific data
            init_tracker: InitTracker::new(None),
            current_turn_id: Vec::new(),
            next_id: Vec::new(),
            current_initiative: 0, 
            next_initiative: 0,
            // initiative_player_map: HashMap::new(),
            combatant_data: HashMap::new(),
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
        }

        let mut blockers = Vec::<Uuid>::new();
        blockers.reserve(self.current_turn_id.len());

        for uuid in &self.current_turn_id
        {
            match self.combatant_data.entry(*uuid) {
                std::collections::hash_map::Entry::Occupied(entry) => {
                    if !entry.get().has_resolved
                    {
                        blockers.push(*uuid);
                    }
                },
                std::collections::hash_map::Entry::Vacant(_) => {unreachable!()},
            }
        }

        if blockers.len() > 0
        {
            return Option::Some(blockers);
        }
        
        Option::None

    }

    pub fn on_deck(self: &Game) -> Option<Vec<Uuid>>
    {
        if self.current_state != State::InitiativePass
        {
            return None;
        }

        if self.next_id.len() == 0
        {
            return None;
        }
        else
        {
            return Some(self.next_id.clone());
        }
    }

    pub fn get_combatants(self: &Game) -> Vec<Uuid>
    {
        let mut combatants = Vec::<Uuid>::new();
        combatants.reserve(self.combatant_data.keys().len());

        for uuid in self.combatant_data.keys()
        {
            combatants.push(*uuid);
        }
        
        return combatants;
    }


    // ******************************************************************************************
    // State change methods

    pub fn end_combat(self: &mut Game)
    {
        self.current_state = State::PreCombat;
        self.current_turn_id.clear();
        self.next_id.clear();
        self.combatant_data.clear();
        self.current_initiative = 0;
        self.next_initiative = 0;
        self.init_tracker.reset();
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
        let combatant_data = CharacterCombatData::new(combatant);

        // TODO: Look up character and review their gear, augs etc. to fill in turns_per_round and/or update any other fields
        self.combatant_data.insert(combatant, combatant_data);

        Ok(())
    }

    pub fn add_combatants(self: &mut Game, mut involved: Vec<Uuid>) -> Result<(), GameError>
    {

        let mut bad_ids = Vec::<String>::new();

        // Set up Characters;
        for id in involved.drain(0..involved.len() - 1)
        {
            match self.add_combatant(id)
            {
                Ok(_) => {},
                Err(_) => bad_ids.push(id.to_string()),
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

    pub fn begin_initiative_roll(self: &mut Game) -> Result<(), GameError>
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

    pub fn add_initiative(self: &mut Game, character_id: Uuid, initiative: i8) -> Result<(), GameError>
    {
        if self.current_state != State::Initiative
        {
            return Err(GameError{
                kind: ErrorKind::InvalidStateAction,
                msg: String::from(format!("The game is not in the initiative phase: you cannot add a new initiative roll."))
            });
        }

        // TODO: scan the ID'd character to 
        if let Some(combat_data) = self.combatant_data.get_mut(&character_id)
        {
            self.init_tracker.add_new_event
            (
                character_id, 
                initiative, 
                combat_data.initiative_passes, 
                combat_data.astral_passes, 
                combat_data.matrix_passes
            );

            combat_data.declared_initiative = true;
        }
        else
        {
            return Err(GameError::new(ErrorKind::NoCombatants, String::from(format!("The id {} does not match any registered combatant.", character_id))));
        }

        Ok(())
    }

    pub fn begin_initiative_passes(self: &mut Game) -> Result<(), GameError>
    {
        for combatant in self.combatant_data.values()
        {
            if !combatant.declared_initiative
            {
                return Err(GameError::new(
                    ErrorKind::InvalidStateAction, 
                    String::from("Not all combatants have supplied their initiative.  Cannot begin passes.")
                ));
            }
        }

        self.current_state = State::InitiativePass;

        return self.initialize_initiatives();
    }

    fn initialize_initiatives(&mut self) -> Result<(), GameError>
    {
        match self.init_tracker.next()
        {
            PassState::PassDone => {
                return Err(GameError::new(
                    ErrorKind::NoCombatants, 
                    String::from("No more initiative passes to be processed.")
                ))
            },
            PassState::Next(top_init) => {
                self.current_initiative = top_init.1;
                self.current_turn_id.push(top_init.0);

                while let PassState::Next(same_turn) = self.init_tracker.next_if_match(self.current_initiative)
                {
                    self.current_turn_id.push(same_turn.0);
                }
            },
            _ => {unreachable!()}
        }

        // And then load the on-deck slot as well.
        match self.init_tracker.next()
        {
            PassState::PassDone => {
                self.next_id.clear();
            },
            PassState::Next(top_init) => {
                self.next_initiative = top_init.1;
                self.next_id.push(top_init.0);

                while let PassState::Next(_) = self.init_tracker.next_if_match(self.next_initiative)
                {
                    self.next_id.push(top_init.0);
                }
                
            },
            _ => {unreachable!()}
        }

        Ok(())
    }

    pub fn advance_initiative_pass(self: &mut Game) -> Result<(), GameError>
    {
        if self.current_state != State::InitiativePass
        {
            return Err(GameError{
                kind: ErrorKind::InvalidStateAction,
                msg: String::from("The game is not in the character turn phase.  You cannot begin an initiative turn.")
            })
        }

        if self.current_turn_id.len() > 0
        {
            return Err(GameError::new
            (
                ErrorKind::UnresolvedCombatant, 
                String::from("There is still at least one initiative turn to process.  Advance the initiative to empty first.")
            ));
        }

        match self.init_tracker.begin_new_pass()
        {
            PassState::Ready => 
            {
                return self.initialize_initiatives();
            },
            PassState::AllDone =>
            {
                return Err(GameError::new(
                    ErrorKind::EndOfInitiativePass,
                    String::from("All characters that can act in this pass have acted.")
                ));
            },
            _ => {unreachable!()}
        }

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
        if self.unresolved_turn()
        {
            return Err(GameError::new(
                ErrorKind::UnresolvedCombatant,
                String::from("Some character has not resolved their turn yet.")
            ));
        }

        // no unready players.  Eject the current set of characters and initiative, advance the on-deck set...
        self.current_initiative = self.next_initiative;
        self.current_turn_id.clear();

        // li'l rotate
        std::mem::swap(&mut self.current_turn_id, &mut self.next_id);

        // and load the next on-deck set.
        if let PassState::Next(on_deck) = self.init_tracker.next()
        {
            self.next_initiative = on_deck.1;
            self.next_id.push(on_deck.0);
            while let PassState::Next(_) = self.init_tracker.next_if_match(self.next_initiative)
            {
                self.next_id.push(on_deck.0);
            }
        }

        Ok(())
    }

    fn unresolved_turn(&mut self) -> bool
    {
        for id in &self.current_turn_id
        {
            if let Some(combat_data) = self.combatant_data.get(&id)
            {
                if !combat_data.has_resolved
                {
                    return true;
                }
            }
        }

        return false;
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
        // let result = self.initiative_player_map.get_mut(&self.current_initiative);
        if !self.combatant_data.contains_key(&actor)
        {
            return Err(GameError::new(
                ErrorKind::EndOfInitiative,
                String::from(format!("The current initiative value {} does not map to any valid combatants.", self.current_initiative))
            ))
        }

        // let current_combatants = result.unwrap();
        

        if self.current_turn_id.contains(&actor) || action_type == ActionType::Free
        {
            match self.combatant_data.entry(actor)
            {
                Entry::Occupied(mut entry) => 
                {
                    let combat_data = entry.get_mut();

                    if action_type != ActionType::Free && combat_data.has_resolved
                    {
                        return Err(GameError::new
                        (
                            ErrorKind::NoAction,
                            String::from("You've already resolved your allowed action.")
                        ))
                    }

                    match action_type
                    {
                        ActionType::Free => {
                            if combat_data.free_actions > 0
                            {
                                combat_data.free_actions -= 1;
                            }
                            else {
                                return Err(GameError::new
                                (
                                    ErrorKind::NoAction, 
                                    String::from("You have already used all of your free actions for this turn.")
                                ));
                            }
                        },
                        ActionType::Simple => {
                            if combat_data.simple_actions > 0
                            {
                                combat_data.simple_actions -= 1;
                                if combat_data.simple_actions == 0
                                {
                                    combat_data.has_resolved = true;
                                }
                            }
                            else {
                                return Err(GameError::new
                                (
                                    ErrorKind::NoAction, 
                                    String::from("You have already used all of your simple actions for this turn.")
                                ));
                            }
                        },
                        ActionType::Complex => {
                            if combat_data.simple_actions < 2 {
                                return Err(GameError::new
                                (
                                    ErrorKind::NoAction,
                                    String::from("You have already taken one simple action - you may not take a complex action too.")
                                ));
                            }
                            if combat_data.complex_actions > 0 
                            {
                                combat_data.complex_actions -= 1;
                                combat_data.has_resolved = true;
                            }
                            else {
                                return Err(GameError::new
                                (
                                    ErrorKind::NoAction, 
                                    String::from("You have already used all of your complex actions for this turn.")
                                ));
                            }
                        },
                    }

                    
                },
                Entry::Vacant(_) => 
                {
                    return Err(GameError::new(
                        ErrorKind::UnknownCastId,
                        String::from(format!("The combat data for combatant {} was not recorded.", actor))
                    ));
                },
            }
        }
        else 
        {
            return Err(GameError::new
            (
                ErrorKind::UnresolvedCombatant,
                String::from(format!("It is not character {}'s turn.", actor))
            ));
        }
        

        
        Ok(())
    }



}

pub struct CharacterCombatData {
    id: Uuid,
    declared_initiative: bool,
    initiative_passes: usize,
    astral_passes: usize,
    matrix_passes: usize,
    // actions: HashMap<ActionType, usize>,
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
            declared_initiative: false,
            initiative_passes: 0, 
            astral_passes: 3,
            matrix_passes: 3,
            free_actions: 1, 
            simple_actions: 2, 
            complex_actions: 1, 
            // actions: HashMap::new(),
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
    Free = 0,
    Simple = 1,
    Complex = 2
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
    use log::debug;
    use uuid::Uuid;

    use crate::game::{game::ActionType, character::{Character, Metatypes}};

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
    pub fn test_waiting_for()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let dorf_id = dorf.id;
        let mork = build_orc();
        let mork_id = mork.id;
        let belf = build_elf();
        let belf_id = belf.id;

        populate!(&mut game, dorf, mork, belf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(dorf_id, 9).is_ok());
        assert!(game.add_initiative(mork_id, 12).is_ok());
        assert!(game.add_initiative(belf_id, 12).is_ok());

        assert!(game.begin_initiative_passes().is_ok());

        assert!(game.take_action(mork_id, ActionType::Complex).is_ok());

        let blockers_option = game.waiting_for();
        assert_ne!(blockers_option, None);
        let blockers = blockers_option.unwrap();
        assert_eq!(blockers.len(), 1);
        let blocking_id = *blockers.get(0).unwrap();
        assert_eq!(blocking_id, belf_id);
    }

    #[test]
    pub fn test_waiting_for_no_one()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let dorf_id = dorf.id;
        let mork = build_orc();
        let mork_id = mork.id;
        let belf = build_elf();
        let belf_id = belf.id;

        populate!(&mut game, dorf, mork, belf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(dorf_id, 9).is_ok());
        assert!(game.add_initiative(mork_id, 12).is_ok());
        assert!(game.add_initiative(belf_id, 12).is_ok());

        assert!(game.begin_initiative_passes().is_ok());

        assert!(game.take_action(mork_id, ActionType::Complex).is_ok());
        assert!(game.take_action(belf_id, ActionType::Complex).is_ok());

        assert_eq!(game.waiting_for(), None);
    }

    #[test]
    pub fn test_waiting_wrong_state()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let dorf_id = dorf.id;
        let mork = build_orc();
        let mork_id = mork.id;
        
        game.add_cast_member(mork);
        game.add_cast_member(dorf);

        assert!(game.waiting_for().is_none());

        assert!(game.add_combatant(dorf_id).is_ok());
        assert!(game.add_combatant(mork_id).is_ok());

        assert!(game.begin_initiative_roll().is_ok());

        assert!(game.waiting_for().is_none());

        assert!(game.add_initiative(dorf_id, 22).is_ok());
        assert!(game.add_initiative(mork_id, 12).is_ok());
        assert!(game.waiting_for().is_none());
    }

    #[test]
    pub fn test_on_deck()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let dorf_id = dorf.id;
        let mork = build_orc();
        let mork_id = mork.id;
        let belf = build_elf();
        let belf_id = belf.id;

        populate!(&mut game, dorf, mork, belf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(dorf_id, 9).is_ok());
        assert!(game.add_initiative(mork_id, 12).is_ok());
        assert!(game.add_initiative(belf_id, 12).is_ok());

        assert!(game.begin_initiative_passes().is_ok());

        let option = game.on_deck();
        assert_ne!(None, option);
        let on_deck = option.unwrap();
        assert_eq!(1, on_deck.len());
        assert!(on_deck.contains(&dorf_id));
    }

    #[test]
    pub fn test_on_deck_wrong_state()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let dorf_id = dorf.id;
        let mork = build_orc();
        let mork_id = mork.id;
        let belf = build_elf();
        let belf_id = belf.id;

        populate!(&mut game, dorf, mork, belf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(dorf_id, 9).is_ok());
        assert!(game.add_initiative(mork_id, 12).is_ok());
        assert!(game.add_initiative(belf_id, 12).is_ok());

        assert_eq!(None, game.on_deck());
    }

    #[test]
    pub fn test_on_deck_none_remain()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let dorf_id = dorf.id;
        let mork = build_orc();
        let mork_id = mork.id;
        let belf = build_elf();
        let belf_id = belf.id;

        populate!(&mut game, dorf, mork, belf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(dorf_id, 9).is_ok());
        assert!(game.add_initiative(mork_id, 12).is_ok());
        assert!(game.add_initiative(belf_id, 12).is_ok());

        assert!(game.begin_initiative_passes().is_ok());

        assert!(game.take_action(mork_id, ActionType::Complex).is_ok());
        assert!(game.take_action(belf_id, ActionType::Complex).is_ok());

        assert!(game.advance_initiative().is_ok());
        assert_eq!(None, game.on_deck());

    }

    #[test]
    pub fn test_get_combatants()
    {
        let mut game = Game::new();

        let mork = build_orc();
        let mork_id = mork.id;
        let dorf = build_dwarf();
        let dorf_id = dorf.id;

        populate!(&mut game, mork, dorf);

        let combatants = game.get_combatants();

        assert_eq!(2, combatants.len());
        assert!(combatants.contains(&mork_id));
        assert!(combatants.contains(&dorf_id));

    }

    #[test]
    pub fn test_get_some_combatants()
    {
        let mut game = Game::new();

        let mork = build_orc();
        let mork_id = mork.id;
        let dorf = build_dwarf();
        let dorf_id = dorf.id;
        let melf = build_elf();

        populate!(&mut game, mork, dorf);
        game.add_cast_member(melf);

        let combatants = game.get_combatants();

        assert_eq!(2, combatants.len());
        assert!(combatants.contains(&mork_id));
        assert!(combatants.contains(&dorf_id));
    }

    #[test]
    pub fn test_get_no_combatants()
    {
        let mut game = Game::new();

        let mork = build_orc();
        let dorf = build_dwarf();
        let melf = build_elf();

        game.add_cast_member(mork);
        game.add_cast_member(dorf);
        game.add_cast_member(melf);

        let combatants = game.get_combatants();

        assert_eq!(0, combatants.len());
    }

    #[test]
    pub fn test_get_combatants_no_cast()
    {
        let game = Game::new();

        let combatants = game.get_combatants();

        assert_eq!(0, combatants.len());
    }

    #[test]
    pub fn test_end_combat()
    {
        let mut game = Game::new();

        let mork = build_orc();
        let mork_id = mork.id;
        let dorf = build_dwarf();
        let dorf_id = dorf.id;
        let melf = build_elf();
        let elf_id = melf.id;

        populate!(&mut game, mork, dorf, melf);

        assert!(game.begin_initiative_roll().is_ok());

        assert!(game.add_initiative(mork_id, 13).is_ok());
        assert!(game.add_initiative(dorf_id, 22).is_ok());
        assert!(game.add_initiative(elf_id, 14).is_ok());

        assert!(game.begin_initiative_passes().is_ok());

        assert_eq!(game.get_combatants().len(), 3);

        game.end_combat();
        assert_eq!(game.get_combatants().len(), 0);
        assert_eq!(game.current_state(), String::from("PreCombat"));
        let on_deck = game.on_deck();
        assert!(on_deck.is_none());
        

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
    pub fn test_adding_multiple_combatants()
    {
        let dorf = build_dwarf();
        let dorf_id = dorf.id;
        let mork = build_orc();
        let mork_id = mork.id;
        let melf = build_elf();
        let melf_id = melf.id;

        let mut game = Game::new();

        game.add_cast_member(dorf);
        game.add_cast_member(mork);
        game.add_cast_member(melf);

        let mut combatants = Vec::<Uuid>::new();
        combatants.push(dorf_id);
        combatants.push(mork_id);
        combatants.push(melf_id);

        assert!(game.add_combatants(combatants).is_ok());

    }

    #[test]
    pub fn test_adding_multiple_unknowns()
    {
        let dorf = build_dwarf();
        let dorf_id = dorf.id;
        let mork = build_orc();
        let mork_id = mork.id;
        let melf = build_elf();
        let melf_id = melf.id;

        let mut game = Game::new();

        game.add_cast_member(dorf);

        let mut combatants = Vec::<Uuid>::new();
        combatants.push(dorf_id);
        combatants.push(mork_id);
        combatants.push(melf_id);

        assert!(game.add_combatants(combatants).is_err());
    }

    #[test]
    pub fn test_initative_transition()
    {
        let melf = Character::new_npc(Metatypes::Elf, String::from("Melf"));
        let melf_id = melf.id;
        let zorc = Character::new_npc(Metatypes::Orc, String::from("Zorc"));
        let zorc_id = zorc.id;
        let mut game = Game::new();

        game.add_cast_member(melf);
        game.add_cast_member(zorc);

        assert!(game.add_combatant(melf_id).is_ok());
        assert!(game.add_combatant(zorc_id).is_ok());

        let result = game.begin_initiative_roll();

        assert!(result.is_ok());
        assert_eq!(game.current_state(), String::from("Initiative Rolls"));
    }

    #[test]
    pub fn test_roll_transition_from_end_of_passes()
    {
        let mut game = Game::new();
        let melf = build_elf();
        let melf_id = melf.id;
        let mork = build_orc();
        let mork_id = mork.id;
        let dorf = build_dwarf();
        let dorf_id = dorf.id;

        populate!(&mut game, melf, mork, dorf);

        // transition from PreCombat
        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(melf_id, 18).is_ok());
        assert!(game.add_initiative(mork_id, 12).is_ok());
        assert!(game.add_initiative(dorf_id, 13).is_ok());
    
        // transition into InitiativePass
        assert!(game.begin_initiative_passes().is_ok());

        // now try to jump right back to initiative roll
        assert!(game.begin_initiative_roll().is_ok());

    }

    #[test]
    pub fn test_initiative_transition_fail()
    {
        let lef = Character::new_npc(Metatypes::Elf, String::from("Lef"));

        let mut game = Game::new();

        game.add_cast_member(lef);

        let result = game.begin_initiative_roll();

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
        assert!(game.add_combatant(lef_id).is_ok());

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

        let mut game = Game::new();
        populate!(&mut game, zorc, dorf);
        assert!(game.begin_initiative_roll().is_ok());

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
        assert!(game.begin_initiative_roll().is_ok());

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
        assert!(game.begin_initiative_roll().is_ok());

        let result = game.add_initiative(Uuid::new_v4(), 2);
        assert!(result.is_err());
    }



    #[test]
    pub fn begin_initiative()
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

        assert!(game.begin_initiative_roll().is_ok());

        assert!(game.add_initiative(zorc_id, 4).is_ok());
        assert!(game.add_initiative(mork_id, 8).is_ok());
        assert!(game.add_initiative(dork_id, 3).is_ok());

        let result = game.begin_initiative_passes();

        assert_eq!(game.current_state(), String::from("Initiative Pass"));
        assert!(result.is_ok());

    }

    #[test]
    pub fn prematurely_begin_initiative()
    {
        init();

        let zorc = build_orc();
        let zorc_id = zorc.id;
        let melf = build_elf();
        let dork = build_dwarf();
        let dork_id = dork.id;

        let mut game = Game::new ();
        populate!(&mut game, zorc, dork, melf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(zorc_id, 1).is_ok());
        assert!(game.add_initiative(dork_id, 15).is_ok());

        let result = game.begin_initiative_passes();

        assert_eq!(game.current_state(), String::from("Initiative Rolls"));
        assert!(result.is_err());
    }

    #[test]
    pub fn advance_init_pass_unresolved_turns()
    {
        init();

        let zorc = build_orc();
        let zorc_id = zorc.id;
        let melf = build_elf();
        let melf_id = melf.id;
        let dork = build_dwarf();
        let dork_id = dork.id;

        let mut game = Game::new();
        populate!(&mut game, zorc, dork, melf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(zorc_id, 23).is_ok());
        assert!(game.add_initiative(melf_id, 20).is_ok());
        assert!(game.add_initiative(dork_id, 33).is_ok());
        assert!(game.begin_initiative_passes().is_ok());
        
        // Attempting to advance to the next initiative pass should result in failure.
        let result = game.advance_initiative_pass();
        assert!(result.is_err());
        debug!("{}", result.unwrap_err().msg);
    }

    #[test]
    pub fn advance_init_pass_no_further_passes()
    {
        init();

        let zorc = build_orc();
        let zorc_id = zorc.id;
        let melf = build_elf();
        let melf_id = melf.id;
        let dork = build_dwarf();
        let dork_id = dork.id;

        let mut game = Game::new();
        populate!(&mut game, zorc, dork, melf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(zorc_id, 23).is_ok());
        assert!(game.add_initiative(melf_id, 20).is_ok());
        assert!(game.add_initiative(dork_id, 33).is_ok());
        assert!(game.begin_initiative_passes().is_ok());

        assert!(game.take_action(dork_id, ActionType::Complex).is_ok());
        assert!(game.advance_initiative().is_ok());
        assert!(game.take_action(zorc_id, ActionType::Complex).is_ok());
        assert!(game.advance_initiative().is_ok());
        assert!(game.take_action(melf_id, ActionType::Complex).is_ok());
        assert!(game.advance_initiative().is_ok());

        assert!(game.advance_initiative_pass().is_err());

    }

    #[test]
    pub fn advance_init_pass_wrong_state()
    {
        init();

        let zorc = build_orc();
        let zorc_id = zorc.id;
        let melf = build_elf();
        let melf_id = melf.id;
        let dork = build_dwarf();
        let dork_id = dork.id;

        let mut game = Game::new();
        populate!(&mut game, zorc, dork, melf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(zorc_id, 23).is_ok());
        assert!(game.add_initiative(melf_id, 20).is_ok());
        assert!(game.add_initiative(dork_id, 33).is_ok());
        
        assert!(game.advance_initiative_pass().is_err());
    }

    #[test]
    pub fn advance_initiative()
    {
        let zorc = build_orc();
        let zorc_id = zorc.id;
        let melf = build_elf();
        let melf_id = melf.id;
        let dork = build_dwarf();
        let dork_id = dork.id;

        let mut game = Game::new();
        populate!(&mut game, zorc, dork, melf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(zorc_id, 23).is_ok());
        assert!(game.add_initiative(melf_id, 20).is_ok());
        assert!(game.add_initiative(dork_id, 33).is_ok());

        assert!(game.begin_initiative_passes().is_ok());

        assert!(game.take_action(dork_id, ActionType::Complex).is_ok());
        assert!(game.advance_initiative().is_ok());
        assert!(game.take_action(zorc_id, ActionType::Complex).is_ok());
        assert!(game.advance_initiative().is_ok());
        assert!(game.take_action(melf_id, ActionType::Complex).is_ok());
        assert!(game.advance_initiative().is_ok());
        
    }

    #[test]
    pub fn advance_initiative_unresolved()
    {
        init();

        let zorc = build_orc();
        let zorc_id = zorc.id;
        let melf = build_elf();
        let melf_id = melf.id;
        let dork = build_dwarf();
        let dork_id = dork.id;

        let mut game = Game::new();
        populate!(&mut game, zorc, dork, melf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(zorc_id, 23).is_ok());
        assert!(game.add_initiative(melf_id, 20).is_ok());
        assert!(game.add_initiative(dork_id, 33).is_ok());

        assert!(game.begin_initiative_passes().is_ok());

        assert!(game.advance_initiative().is_err());


    }

    // TODO: Add tests to ensure advance_initiative_pass succeeds when at least one character or event occurs on the next pass.
    // Currently cannot do that because there's no Game infrastructure to calculate or influence the pass count for any character,
    // or add some on-next-pass event.

    #[test]
    pub fn take_simple_action()
    {
        init();

        let zorc = build_orc();
        let zorc_id = zorc.id;
        let melf = build_elf();
        let melf_id = melf.id;
        let dork = build_dwarf();
        let dork_id = dork.id;

        let mut game = Game::new();
        populate!(&mut game, zorc, dork, melf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(zorc_id, 23).is_ok());
        assert!(game.add_initiative(melf_id, 20).is_ok());
        assert!(game.add_initiative(dork_id, 33).is_ok());

        assert!(game.begin_initiative_passes().is_ok());

        assert!(game.take_action(dork_id, ActionType::Simple).is_ok());
        assert!(game.take_action(dork_id, ActionType::Complex).is_err());
        assert!(game.advance_initiative().is_err());
        assert!(game.take_action(dork_id, ActionType::Simple).is_ok());
        assert!(game.take_action(dork_id, ActionType::Simple).is_err());
        assert!(game.advance_initiative().is_ok());
    }

    #[test]
    pub fn take_complex_action()
    {
        init();

        let zorc = build_orc();
        let zorc_id = zorc.id;
        let melf = build_elf();
        let melf_id = melf.id;
        let dork = build_dwarf();
        let dork_id = dork.id;

        let mut game = Game::new();
        populate!(&mut game, zorc, dork, melf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(zorc_id, 23).is_ok());
        assert!(game.add_initiative(melf_id, 20).is_ok());
        assert!(game.add_initiative(dork_id, 33).is_ok());

        assert!(game.begin_initiative_passes().is_ok());

        assert!(game.take_action(dork_id, ActionType::Complex).is_ok());
        assert!(game.take_action(dork_id, ActionType::Simple).is_err());
        assert!(game.take_action(dork_id, ActionType::Complex).is_err());
        assert!(game.advance_initiative().is_ok());
    }

    #[test]
    pub fn take_free_and_simple_action()
    {
        init();

        let zorc = build_orc();
        let zorc_id = zorc.id;
        let melf = build_elf();
        let melf_id = melf.id;
        let dork = build_dwarf();
        let dork_id = dork.id;

        let mut game = Game::new();
        populate!(&mut game, zorc, dork, melf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(zorc_id, 23).is_ok());
        assert!(game.add_initiative(melf_id, 20).is_ok());
        assert!(game.add_initiative(dork_id, 33).is_ok());

        assert!(game.begin_initiative_passes().is_ok());

        assert!(game.take_action(dork_id, ActionType::Free).is_ok());
        assert!(game.take_action(dork_id, ActionType::Simple).is_ok());
        assert!(game.take_action(dork_id, ActionType::Simple).is_ok());
        assert!(game.advance_initiative().is_ok());
    }

    #[test]
    pub fn take_free_and_complex_action()
    {
        init();

        let zorc = build_orc();
        let zorc_id = zorc.id;
        let melf = build_elf();
        let melf_id = melf.id;
        let dork = build_dwarf();
        let dork_id = dork.id;

        let mut game = Game::new();
        populate!(&mut game, zorc, dork, melf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(zorc_id, 23).is_ok());
        assert!(game.add_initiative(melf_id, 20).is_ok());
        assert!(game.add_initiative(dork_id, 33).is_ok());

        assert!(game.begin_initiative_passes().is_ok());

        assert!(game.take_action(dork_id, ActionType::Free).is_ok());
        assert!(game.take_action(dork_id, ActionType::Complex).is_ok());
        assert!(game.advance_initiative().is_ok());
    }

    #[test]
    pub fn take_free_on_anothers_turn()
    {
        init();

        let zorc = build_orc();
        let zorc_id = zorc.id;
        let melf = build_elf();
        let melf_id = melf.id;
        let dork = build_dwarf();
        let dork_id = dork.id;

        let mut game = Game::new();
        populate!(&mut game, zorc, dork, melf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(zorc_id, 23).is_ok());
        assert!(game.add_initiative(melf_id, 20).is_ok());
        assert!(game.add_initiative(dork_id, 33).is_ok());

        assert!(game.begin_initiative_passes().is_ok());

        assert!(game.take_action(melf_id, ActionType::Free).is_ok());
        assert!(game.take_action(dork_id, ActionType::Simple).is_ok());
        assert!(game.take_action(dork_id, ActionType::Simple).is_ok());
        assert!(game.advance_initiative().is_ok());
    }

    #[test]
    pub fn take_simple_on_anothers_turn()
    {
        init();

        let zorc = build_orc();
        let zorc_id = zorc.id;
        let melf = build_elf();
        let melf_id = melf.id;
        let dork = build_dwarf();
        let dork_id = dork.id;

        let mut game = Game::new();
        populate!(&mut game, zorc, dork, melf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(zorc_id, 23).is_ok());
        assert!(game.add_initiative(melf_id, 20).is_ok());
        assert!(game.add_initiative(dork_id, 33).is_ok());

        assert!(game.begin_initiative_passes().is_ok());

        assert!(game.take_action(dork_id, ActionType::Free).is_ok());
        assert!(game.take_action(melf_id, ActionType::Simple).is_err());
        assert!(game.take_action(zorc_id, ActionType::Simple).is_err());
        assert!(game.advance_initiative().is_err());
    }

    #[test]
    pub fn take_complex_on_anothers_turn()
    {
        init();

        let zorc = build_orc();
        let zorc_id = zorc.id;
        let melf = build_elf();
        let melf_id = melf.id;
        let dork = build_dwarf();
        let dork_id = dork.id;

        let mut game = Game::new();
        populate!(&mut game, zorc, dork, melf);

        assert!(game.begin_initiative_roll().is_ok());
        assert!(game.add_initiative(zorc_id, 23).is_ok());
        assert!(game.add_initiative(melf_id, 20).is_ok());
        assert!(game.add_initiative(dork_id, 33).is_ok());

        assert!(game.begin_initiative_passes().is_ok());

        assert!(game.take_action(dork_id, ActionType::Free).is_ok());
        assert!(game.take_action(melf_id, ActionType::Complex).is_err());
        assert!(game.advance_initiative().is_err());
    }


}