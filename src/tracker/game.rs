use std::{collections::{HashMap, hash_map::Entry}, sync::Arc};

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

    cast: HashMap<Uuid, Arc<Character>>,

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

    pub fn add_cast_member(self: &mut Game, mut cast_member: Character) -> Uuid
    {
        let id = Uuid::new_v4();
        cast_member.id = id;
        self.cast.insert(id, Arc::new(cast_member));

        return id;
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
        if self.current_state != State::ActionRound
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

    pub fn currently_up(self: &Game) -> Option<Vec<Uuid>>
    {
        if self.current_turn_id.len() > 0
        {
            Some(self.current_turn_id.clone())
        }
        else
        {
            None
        }
    }

    pub fn on_deck(self: &Game) -> Option<Vec<Uuid>>
    {
        if self.current_state != State::ActionRound
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

    pub fn collect_all_remaining_events(self: &mut Game) -> Option<HashMap<i8, Vec<Uuid>>>
    {
        let events = self.init_tracker.get_ordered_inits();

        if events.len() + self.current_turn_id.len() + self.next_id.len() == 0
        {
            return None;
        }

        let mut collection = HashMap::<i8, Vec<Uuid>>::new();

        if !self.current_turn_id.is_empty()
        {
            match collection.entry(self.current_initiative) 
            {
                Entry::Occupied(mut entry) => 
                {
                    let temp = entry.get_mut();
                    for id in &self.current_turn_id
                    {
                        temp.push(*id);
                    }
                },
                Entry::Vacant(new_entry) =>
                {
                    let mut temp = Vec::<Uuid>::new();
                    for id in &self.current_turn_id
                    {
                        temp.push(*id);
                    }
                    new_entry.insert(temp);
                },
            }
        }

        if !self.next_id.is_empty()
        {
            match collection.entry(self.next_initiative) 
            {
                Entry::Occupied(mut entry) => 
                {
                    let temp = entry.get_mut();
                    for id in &self.next_id
                    {
                        temp.push(*id);
                    }
                },
                Entry::Vacant(new_entry) =>
                {
                    let mut temp = Vec::<Uuid>::new();
                    for id in &self.next_id
                    {
                        temp.push(*id);
                    }
                    new_entry.insert(temp);
                },
            }
        }

        for (init, event) in events
        {
            match collection.entry(init)
            {
                Entry::Occupied(mut entry) => 
                {
                    entry.get_mut().push(event)
                },
                Entry::Vacant(new_entry) =>
                {
                    let mut temp = Vec::<Uuid>::new();
                    temp.push(event);
                    new_entry.insert(temp);
                },
            }
        }

        return Some(collection);
    }

    pub fn get_current_init(self: &Game) -> Option<i8>
    {
        if self.current_state != State::ActionRound
        {
            None
        }
        else if self.current_turn_id.len() == 0
        {
            None
        }
        else
        {
            Some(self.current_initiative)
        }
    }

    pub fn get_next_init(self: &Game) -> Option<i8>
    {
        if self.current_state != State::ActionRound
        {
            None
        }
        else if self.next_id.len() == 0
        {
            None
        }
        else
        {
            Some(self.next_initiative)
        }
    }

    pub fn get_all_remaining_initiatives(self: &mut Game) -> Option<Vec<i8>>
    {
        let mut initiatives = Vec::<i8>::new();

        for (init, _id) in self.init_tracker.get_ordered_inits()
        {
            initiatives.push(init);
        }

        if self.next_id.len() > 0
        {
            initiatives.push(self.next_initiative);
        }

        if self.current_turn_id.len() > 0
        {
            initiatives.push(self.current_initiative);
        }

        initiatives.dedup();

        if initiatives.len() == 0
        {
            None
        }
        else
        {
            return Some(initiatives)
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

    pub fn are_any_initiatives_outstanding(self: &mut Game) -> bool
    {
        for combatant in (&self.combatant_data).values() {
            if !combatant.declared_initiative
            {
                return true;
            }
        }

        return false;
    }

    pub fn collect_undeclared_initiatives(self: &mut Game) -> Vec<Uuid>
    {
        let mut undeclared = Vec::<Uuid>::new();

        for (id, combatant) in &self.combatant_data
        {
            if !combatant.declared_initiative
            {
                undeclared.push(*id);
            }
        }

        return undeclared;
    }

    pub fn get_cast(self: &Game) -> Vec<Arc<Character>>
    {
        let mut result = Vec::new();
        for (_id, sheet) in &self.cast
        {
            result.push(sheet.clone());
        }

        return result;
    }

    pub fn get_npcs(self: &Game) -> Vec<Arc<Character>>
    {
        self.filter_cast_by(false)
    }

    pub fn get_pcs(self: &Game) -> Vec<Arc<Character>>
    {
        self.filter_cast_by(true)
    }

    fn filter_cast_by(self: &Game, player_owned: bool) -> Vec<Arc<Character>>
    {
        let mut result = Vec::new();
        for (_id, sheet) in &self.cast
        {
            if sheet.player_character == player_owned
            {
                result.push(sheet.clone());
            }
        }

        return result;
    }

    pub fn get_cast_by_id(self: &Game, char_id: &Uuid) -> Option<Arc<Character>>
    {
        if self.cast.contains_key(&char_id)
        {
            Some(self.cast.get(&char_id).unwrap().clone())
        }
        else
        {
            None
        }
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
        let combatant_data = CharacterCombatData::new();

        // TODO: Look up character and review their gear, augs etc. to fill in turns_per_round and/or update any other fields
        self.combatant_data.insert(combatant, combatant_data);

        Ok(())
    }

    pub fn add_combatants(self: &mut Game, involved: Vec<Uuid>) -> Result<(), GameError>
    {

        let mut bad_ids = Vec::<String>::new();

        // Set up Characters;
        // for id in involved.drain(0..involved.len() - 1)
        for id in involved.into_iter()
        {
            match self.add_combatant(id)
            {
                Ok(_) => {},
                Err(_) => bad_ids.push(id.to_string()),
            }
        }
        
        if !bad_ids.is_empty() {
            let missing_ids = bad_ids.join(", ");
            return Err(GameError{
                kind: ErrorKind::UnknownCastId,
                msg: String::from(format!("The character(s) with id(s) {} is not registered as a cast member of this adventure.", missing_ids))
            });
        }

        Ok(())
    }

    pub fn start_initiative_phase(self: &mut Game) -> Result<(), GameError>
    {
        debug!("Starting initiative.");
        if self.current_state != State::PreCombat && self.current_state != State::ActionRound
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
                ErrorKind::UnknownCastId, String::from("You may not begin an initiative round if no one is going to fight.")
            ))
        }

        if self.unresolved_turn() || self.on_deck().is_some()
        {
            debug!("There are still unresolved events this turn.");
            return Err(GameError::new
            (
                ErrorKind::UnresolvedCombatant, String::from("There are still unresolved events this turn - you may not start the next turn.")
            ))
        }

        self.current_state = State::Initiative;
        self.reset_actions();
        self.init_tracker.end_turn();
    

        Ok(())
    }

    pub fn accept_initiative_roll(self: &mut Game, character_id: Uuid, initiative: i8) -> Result<(), GameError>
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
            return Err(GameError::new(ErrorKind::UnknownCastId, String::from(format!("The id {} does not match any registered combatant.", character_id))));
        }

        Ok(())
    }

    pub fn start_combat_rounds(self: &mut Game) -> Result<(), GameError>
    {
        if self.current_state != State::Initiative
        {
            return Err(GameError::new(
                ErrorKind::InvalidStateAction,
                String::from("Not in the initiative state.  Cannot advance to combat.")
            ));
        }

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

        self.initialize_initiatives()?;
        self.current_state = State::ActionRound;

        return Ok(()); 
    }

    fn initialize_initiatives(&mut self) -> Result<(), GameError>
    {
        match self.init_tracker.next()
        {
            PassState::PassDone => {
                return Err(GameError::new(
                    ErrorKind::UnknownCastId, 
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

    pub fn next_initiative_pass(self: &mut Game) -> Result<(), GameError>
    {
        if self.current_state != State::ActionRound
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

    pub fn advance_round(self: &mut Game) -> Result<(), GameError>
    {
        if self.current_state != State::ActionRound
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
        else
        {
            self.next_id.clear();
        }

        if self.current_turn_id.len() == 0
        {
            return Err(GameError::new(ErrorKind::EndOfInitiative, String::from("End of initiative order.")))
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

        if self.current_state != State::ActionRound
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

    fn reset_actions(&mut self)
    {
        for (_id, data) in &mut self.combatant_data
        {
            data.reset();
        }
    }

}

pub struct CharacterCombatData {
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
    pub fn new()->CharacterCombatData {
        CharacterCombatData 
        { 
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
    ActionRound,
    // PostRound,
    // Other,
}

impl State {
    pub fn to_string(self: &State) -> String
    {
        match self {
            State::PreCombat => String::from("PreCombat"),
            State::Initiative => String::from("Initiative Rolls"),
            State::ActionRound => String::from("Initiative Pass"),
            // State::PostRound => String::from("End Of Round"),
            // State::Other => String::from("Other"),
        }
    }
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, Copy)]
pub enum ActionType {
    Free = 0,
    Simple = 1,
    Complex = 2
}

#[derive(Debug)]
pub struct GameError {
    pub kind: ErrorKind,
    pub msg: String,
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

    use crate::tracker::{game::{ActionType}, character::{Character, Metatypes}};

    use super::Game;

    pub fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    macro_rules! populate {
        ($game:expr, $($char:expr),*) => {
            {
                let local_game: &mut Game = $game;
                let mut ids = Vec::<Uuid>::new();

                $(let char_id:Uuid = local_game.add_cast_member($char); if let Err(_) = local_game.add_combatant(char_id) {panic!();}; ids.push(char_id);)*

                ids
            }
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
    pub fn when_a_game_is_built_it_starts_in_pre_combat_state_with_empty_cast_and_combat_sets()
    {
        let mut game = Game::new();

        assert_eq!(game.current_state(), String::from("PreCombat"));
        assert_eq!(game.waiting_for(), None);
        assert_eq!(game.on_deck(), None);
        assert_eq!(game.get_combatants().len(), 0);
    }

    #[test]
    pub fn adding_a_cast_member_to_a_game_increases_the_cast_size_by_one()
    {

        let cast_member = Character::new_pc(Metatypes::Human, String::from("Demo"));
        let mut game: Game = Game::new();

        let pre_add_size = game.cast_size();
        game.add_cast_member(cast_member);

        assert_eq!(game.cast_size(), pre_add_size + 1);
        
    }

    #[test]
    pub fn removing_a_cast_member_decreases_cast_size_by_one()
    {
        let cast_member = Character::new_pc(Metatypes::Elf, String::from("Delfmo"));
        //  cast_member.id;
        let mut game: Game = Game::new();

        let id = game.add_cast_member(cast_member);

        let pre_remove_size = game.cast_size();
        game.retire_cast_member(id);
        assert_eq!(game.cast_size(), pre_remove_size - 1); 
    }

    #[test]
    pub fn all_cast_members_uuids_can_be_retrieved_at_any_time()
    {
        let mut mork = build_orc();
        let mut dorf = build_dwarf();
        let mut melf = build_elf();

        mork.player_character = true;
        dorf.player_character = false;
        melf.player_character = true;

        let mut game = Game::new();

        let mork_id = game.add_cast_member(mork);
        let dorf_id = game.add_cast_member(dorf);
        let melf_id = game.add_cast_member(melf);

        let cast = game.get_cast();
        let ids = vec![mork_id, dorf_id, melf_id];

        assert!(cast.len() == 3);
        assert!(ids.contains(&cast.get(0).unwrap().id));
        assert!(ids.contains(&cast.get(1).unwrap().id));
        assert!(ids.contains(&cast.get(2).unwrap().id));
        
    }

    #[test]
    pub fn get_cast_generates_empty_vec_if_no_characters_added()
    {
        let game = Game::new();

        let ids = game.get_cast();
        assert!(ids.is_empty());
    }

    #[test]
    pub fn get_npcs_returns_partial_set_of_characters()
    {
        let mut mork = build_orc();
        let mut dorf = build_dwarf();
        let mut melf = build_elf();

        mork.player_character = true;
        dorf.player_character = false;
        melf.player_character = true;

        let mut game = Game::new();

        let _mork_id = game.add_cast_member(mork);
        let dorf_id = game.add_cast_member(dorf);
        let _melf_id = game.add_cast_member(melf);

        let cast = game.get_npcs();

        assert!(cast.len() == 1);
        assert!(cast.get(0).unwrap().id == dorf_id);
    }

    #[test]
    pub fn if_no_cast_members_are_npcs_get_npcs_returns_empty_vec()
    {
        let mut mork = build_orc();
        let mut dorf = build_dwarf();
        let mut melf = build_elf();

        mork.player_character = true;
        dorf.player_character = true;
        melf.player_character = true;

        let mut game = Game::new();

        let _mork_id = game.add_cast_member(mork);
        let _dorf_id = game.add_cast_member(dorf);
        let _melf_id = game.add_cast_member(melf);

        let ids = game.get_npcs();

        assert!(ids.is_empty());
    }

    #[test]
    pub fn get_pcs_returns_partial_set_of_characters()
    {
        let mut mork = build_orc();
        let mut dorf = build_dwarf();
        let mut melf = build_elf();

        mork.player_character = true;
        dorf.player_character = false;
        melf.player_character = true;

        let mut game = Game::new();

        let mork_id = game.add_cast_member(mork);
        let _dorf_id = game.add_cast_member(dorf);
        let melf_id = game.add_cast_member(melf);

        let cast = game.get_pcs();
        let ids = vec![mork_id, melf_id];

        assert!(ids.len() == 2);
        assert!(ids.contains(&cast.get(0).unwrap().id));
        assert!(ids.contains(&cast.get(1).unwrap().id));
    }

    #[test]
    pub fn if_no_cast_members_are_pcs_get_npcs_returns_empty_vec()
    {
        let mut mork = build_orc();
        let mut dorf = build_dwarf();
        let mut melf = build_elf();

        mork.player_character = false;
        dorf.player_character = false;
        melf.player_character = false;

        let mut game = Game::new();

        let _mork_id = game.add_cast_member(mork);
        let _dorf_id = game.add_cast_member(dorf);
        let _melf_id = game.add_cast_member(melf);

        let ids = game.get_pcs();

        assert!(ids.is_empty());
    }

    #[test]
    pub fn get_cast_by_id_returns_some_with_character_if_character_id_is_in_cast()
    {
        let mut mork = build_orc();
        let mut dorf = build_dwarf();
        let mut melf = build_elf();

        mork.player_character = false;
        dorf.player_character = true;
        melf.player_character = false;

        let mut game = Game::new();

        let _mork_id = game.add_cast_member(mork);
        let dorf_id = game.add_cast_member(dorf);
        let _melf_id = game.add_cast_member(melf);

        assert!(game.get_cast_by_id(&dorf_id).is_some());
        let found_dorf = game.get_cast_by_id(&dorf_id).unwrap();
        assert_eq!(dorf_id, found_dorf.id);
        
    }

    #[test]
    pub fn get_cast_by_id_returns_none_if_character_id_is_not_in_cast()
    {
        let mut mork = build_orc();
        let mut dorf = build_dwarf();
        let mut melf = build_elf();

        mork.player_character = false;
        dorf.player_character = true;
        melf.player_character = false;

        let mut game = Game::new();

        let _mork_id = game.add_cast_member(mork);
        let _dorf_id = game.add_cast_member(dorf);
        let _melf_id = game.add_cast_member(melf);

        assert!(game.get_cast_by_id(&Uuid::new_v4()).is_none());
    }

    #[test]
    pub fn currently_up_produces_full_list_of_combatants_with_current_initiative()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 14).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 14).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        match game.currently_up() 
        {
            Some(chars) => 
            {
                assert!(chars.contains(ids.get(1).unwrap()));
                assert!(chars.contains(ids.get(2).unwrap()));
                assert!(!chars.contains(ids.get(0).unwrap()));
            },
            None => {panic!("currently_up test failed: should have produced ids for both mork and belf.")}
        }
    }

    #[test]
    pub fn currently_up_excludes_combatants_before_and_after_current_init()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.start_initiative_phase().is_ok());

        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 14).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 16).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(2).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_ok());

        match game.currently_up()
        {
            Some(chars) =>
            {
                assert!(chars.len() == 1);
                assert!(chars.contains(ids.get(1).unwrap()));
            }
            None => {panic!("currently_up_excludes failed: should have returned at least one ID.")}
        }
    }

    #[test]
    pub fn currently_up_is_empty_before_action_rounds()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.start_initiative_phase().is_ok());

        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 14).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 16).is_ok());

        assert!(game.currently_up().is_none());

    }

    #[test]
    pub fn waiting_for_produces_a_list_of_ids_for_characters_who_have_not_acted_on_the_current_combat_turn()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        // let dorf_id = dorf.id;
        let mork = build_orc();
        // let mork_id = mork.id;
        let belf = build_elf();
        // let belf_id = belf.id;

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 9).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 12).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_ok());

        let blockers_option = game.waiting_for();
        assert_ne!(blockers_option, None);
        let blockers = blockers_option.unwrap();
        assert_eq!(blockers.len(), 1);
        let blocking_id = *blockers.get(0).unwrap();
        assert_eq!(blocking_id, *ids.get(2).unwrap());
    }

    #[test]
    pub fn waiting_for_produces_empty_list_if_all_characters_in_combat_turn_have_acted()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 9).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 12).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_ok());
        assert!(game.take_action(*ids.get(2).unwrap(), ActionType::Complex).is_ok());

        assert_eq!(game.waiting_for(), None);
    }

    #[test]
    pub fn waiting_for_produces_none_if_not_in_combat_turns_phase()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        
        let mork_id = game.add_cast_member(mork);
        let dorf_id = game.add_cast_member(dorf);

        assert!(game.waiting_for().is_none());

        assert!(game.add_combatant(dorf_id).is_ok());
        assert!(game.add_combatant(mork_id).is_ok());

        assert!(game.start_initiative_phase().is_ok());

        assert!(game.waiting_for().is_none());

        assert!(game.accept_initiative_roll(dorf_id, 22).is_ok());
        assert!(game.accept_initiative_roll(mork_id, 12).is_ok());
        assert!(game.waiting_for().is_none());
    }

    #[test]
    pub fn on_deck_returns_list_of_ids_whose_action_turn_follows_current()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 9).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 12).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        let option = game.on_deck();
        assert_ne!(None, option);
        let on_deck = option.unwrap();
        assert_eq!(1, on_deck.len());
        assert!(on_deck.contains(&ids.get(0).unwrap()));
    }

    #[test]
    pub fn on_deck_produces_none_if_game_not_processing_combat_turns()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 9).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 12).is_ok());

        assert_eq!(None, game.on_deck());
    }

    #[test]
    pub fn on_deck_returns_none_if_no_combatants_remain_to_act_on_this_turn()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 9).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 12).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_ok());
        assert!(game.take_action(*ids.get(2).unwrap(), ActionType::Complex).is_ok());

        assert!(game.advance_round().is_ok());
        assert_eq!(None, game.on_deck());

    }

    #[test]
    pub fn collect_all_events_returns_all_initiatives_and_associated_events()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 9).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 12).is_ok());

        let some = game.collect_all_remaining_events();
        assert!(some.is_some());
        let events = some.unwrap();
        assert!(events.len() == 2);
        assert!(events.contains_key(&12));
        assert!(events.contains_key(&9));
        assert!(events.get(&12).unwrap().len() == 2);
        assert!(events.get(&12).unwrap().contains(ids.get(1).unwrap()));
        assert!(events.get(&12).unwrap().contains(ids.get(2).unwrap()));
        assert!(events.get(&9).unwrap().len() == 1);
        assert!(events.get(&9).unwrap().contains(ids.get(0).unwrap()));
    }

    #[test]
    pub fn collect_all_events_operates_in_all_phases()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.collect_all_remaining_events().is_none());

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 9).is_ok());
        assert!(game.collect_all_remaining_events().is_some());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(game.collect_all_remaining_events().is_some());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 12).is_ok());
        assert!(game.collect_all_remaining_events().is_some());

        assert!(game.start_combat_rounds().is_ok());
        assert!(game.collect_all_remaining_events().is_some());

        let events = game.collect_all_remaining_events().unwrap();
        assert!(events.len() == 2);
        assert!(events.contains_key(&12));
        assert!(events.contains_key(&9));
        assert!(events.get(&12).unwrap().len() == 2);
        assert!(events.get(&12).unwrap().contains(ids.get(1).unwrap()));
        assert!(events.get(&12).unwrap().contains(ids.get(2).unwrap()));
        assert!(events.get(&9).unwrap().len() == 1);
        assert!(events.get(&9).unwrap().contains(ids.get(0).unwrap()));
    }

    #[test]
    pub fn collect_all_events_captures_only_turns_that_have_not_fully_resolved()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.collect_all_remaining_events().is_none());

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 9).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 15).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(2).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_ok());
        
        match game.collect_all_remaining_events()
        {
            Some(events) => 
            {
                assert!(events.len() == 2);
                assert!(events.contains_key(&12));
                assert!(events.contains_key(&9));
                assert!(events.get(&12).unwrap().contains(ids.get(1).unwrap()));
                assert!(events.get(&9).unwrap().contains(ids.get(0).unwrap()));
            },
            None => {panic!("Should have been at least two initiative rolls and events.");}
        }
        
    }

    #[test]
    pub fn calling_get_current_init_before_starting_combat_generates_none()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 9).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 15).is_ok());

        assert!(game.get_current_init().is_none());
    }

    #[test]
    pub fn calling_get_current_init_after_starting_combat_generates_current_init()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 9).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 15).is_ok());
        assert!(game.start_combat_rounds().is_ok());

        assert!(game.get_current_init().is_some());
        assert!(game.get_current_init().unwrap() == 15);
    }

    #[test]
    pub fn calling_get_next_init_before_starting_combat_generates_none()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 9).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 15).is_ok());

        assert!(game.get_next_init().is_none());
    }

    #[test]
    pub fn calling_get_next_init_after_starting_combat_generates_next_init()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 9).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 15).is_ok());
        assert!(game.start_combat_rounds().is_ok());

        assert!(game.get_next_init().is_some());
        assert!(game.get_next_init().unwrap() == 12);
    }

    #[test]
    pub fn calling_get_next_init_on_last_turn_generates_none()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 9).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 15).is_ok());
        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(2).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_ok());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_ok());
        
        assert!(game.get_next_init().is_none());
    }

    #[test]
    pub fn get_all_remaining_initiatives_will_retrieve_all_unresolved_initiatives()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 9).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 15).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        assert!(game.get_all_remaining_initiatives().is_some());
        assert!(game.get_all_remaining_initiatives().unwrap().len() == 3);
    }

    #[test]
    pub fn advancing_turn_will_remove_advanced_initiative_from_get_all_unresolved_list()
    {
        let mut game = Game::new();
        let dorf = build_dwarf();
        let mork = build_orc();
        let belf = build_elf();

        let ids = populate!(&mut game, dorf, mork, belf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 9).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 15).is_ok());

        assert!(game.get_all_remaining_initiatives().is_some());
        assert!(game.get_all_remaining_initiatives().unwrap().len() == 3);

        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(2).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_ok());
        assert!(game.get_all_remaining_initiatives().is_some());
        assert!(game.get_all_remaining_initiatives().unwrap().len() == 2);
        assert!(game.get_all_remaining_initiatives().unwrap().contains(&9));
        assert!(game.get_all_remaining_initiatives().unwrap().contains(&12));

        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_ok());
        assert!(game.get_all_remaining_initiatives().is_some());
        assert!(game.get_all_remaining_initiatives().unwrap().len() == 1);
        assert!(game.get_all_remaining_initiatives().unwrap().contains(&9));

        assert!(game.take_action(*ids.get(0).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_err());
        assert!(game.get_all_remaining_initiatives().is_none());

    }

    #[test]
    pub fn get_combatants_returns_list_of_all_participants_in_the_fight()
    {
        let mut game = Game::new();

        let mork = build_orc();
        let dorf = build_dwarf();

        let ids = populate!(&mut game, mork, dorf);

        let combatants = game.get_combatants();

        assert_eq!(2, combatants.len());
        assert!(combatants.contains(ids.get(0).unwrap()));
        assert!(combatants.contains(ids.get(1).unwrap()));

    }

    #[test]
    pub fn get_combatants_returns_only_cast_members_registered_in_current_combat_session()
    {
        let mut game = Game::new();

        let mork = build_orc();
        let dorf = build_dwarf();
        let melf = build_elf();

        let ids = populate!(&mut game, mork, dorf);
        game.add_cast_member(melf);

        let combatants = game.get_combatants();

        assert_eq!(2, combatants.len());
        assert!(combatants.contains(ids.get(0).unwrap()));
        assert!(combatants.contains(ids.get(1).unwrap()));
    }

    #[test]
    pub fn when_get_combatants_is_called_with_no_active_combatants_an_empty_list_is_returned()
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
    pub fn get_combatants_is_independent_of_cast_list()
    {
        let mut game = Game::new();

        let combatants = game.get_combatants();
        assert_eq!(0, combatants.len());

        let mork = build_orc();
        let dorf = build_dwarf();
        let melf = build_elf();

        game.add_cast_member(mork);
        game.add_cast_member(dorf);
        game.add_cast_member(melf);

        let combatants = game.get_combatants();
        assert_eq!(0, combatants.len());

        assert_eq!(0, combatants.len());
    }

    #[test]
    pub fn when_combat_is_ended_the_game_resets_the_combatants_list_and_resets_game_state()
    {
        let mut game = Game::new();

        let mork = build_orc();
        let dorf = build_dwarf();
        let melf = build_elf();

        let ids = populate!(&mut game, mork, dorf, melf);

        assert!(game.start_initiative_phase().is_ok());

        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 13).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 22).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 14).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        assert_eq!(game.get_combatants().len(), 3);

        game.end_combat();
        assert_eq!(game.get_combatants().len(), 0);
        assert_eq!(game.current_state(), String::from("PreCombat"));
        let on_deck = game.on_deck();
        assert!(on_deck.is_none());
        

    }

    #[test]
    pub fn adding_non_cast_member_to_combat_results_in_unrecognized_character_error()
    {
        let mut game = Game::new();

        let combatant_id = Uuid::new_v4();

        let result = game.add_combatant(combatant_id);

        assert!(result.is_err());
        match result
        {
            Ok(_) => {panic!("This should not have succeeded.")},
            Err(err) => 
            {
                match err.kind
                {
                    crate::tracker::game::ErrorKind::UnknownCastId => {},
                    _ => {panic!("Test expected different error type (UnknownCastId)");}
                }
            },
        }
    }

    #[test]
    pub fn any_real_cast_member_may_be_added_to_combat()
    {
        let dorf = Character::new_npc(Metatypes::Dwarf, String::from("Dorf"));
        let torll = Character::new_npc(Metatypes::Troll, String::from("Torll"));
        let mut game = Game::new();

        let combatant_id = game.add_cast_member(dorf);
        game.add_cast_member(torll);

        let result = game.add_combatant(combatant_id);

        assert!(result.is_ok());
    }

    #[test]
    pub fn multiple_combatants_may_be_added_in_a_vector_of_ids()
    {
        let dorf = build_dwarf();
        let mork = build_orc();
        let melf = build_elf();

        let mut game = Game::new();

        let dorf_id = game.add_cast_member(dorf);
        let mork_id = game.add_cast_member(mork);
        let melf_id = game.add_cast_member(melf);

        let mut combatants = Vec::<Uuid>::new();
        combatants.push(dorf_id);
        combatants.push(mork_id);
        combatants.push(melf_id);

        assert!(game.add_combatants(combatants).is_ok());

    }

    #[test]
    pub fn any_unknown_character_ids_in_vec_cause_unknown_character_error_from_add_combatants()
    {
        let dorf = build_dwarf();
        let _mork = build_orc();
        let _melf = build_elf();

        let mut game = Game::new();

        let dorf_id = game.add_cast_member(dorf);

        let mut combatants = Vec::<Uuid>::new();
        combatants.push(dorf_id);
        combatants.push(Uuid::new_v4());
        combatants.push(Uuid::new_v4());

        match game.add_combatants(combatants)
        {
            Ok(_) => {panic!("This should not have succeeded.")},
            Err(err) => 
            {
                match err.kind
                {
                    crate::tracker::game::ErrorKind::UnknownCastId => {},
                    _ => {panic!("Test expected different error type (UnknownCastId)");}
                }
            },
        }
    }

    #[test]
    pub fn when_a_game_has_combatants_it_may_enter_into_initiative_phase()
    {
        let melf = Character::new_npc(Metatypes::Elf, String::from("Melf"));
        let zorc = Character::new_npc(Metatypes::Orc, String::from("Zorc"));
        let mut game = Game::new();

        let melf_id = game.add_cast_member(melf);
        let zorc_id = game.add_cast_member(zorc);

        assert!(game.add_combatant(melf_id).is_ok());
        assert!(game.add_combatant(zorc_id).is_ok());

        let result = game.start_initiative_phase();

        assert!(result.is_ok());
        assert_eq!(game.current_state(), String::from("Initiative Rolls"));
    }

    #[test]
    pub fn when_all_combatants_have_supplied_initiative_combat_round_phase_may_start()
    {
        let mut game = Game::new();
        let melf = build_elf();
        let mork = build_orc();
        let dorf = build_dwarf();

        let ids = populate!(&mut game, melf, mork, dorf);

        // transition from PreCombat
        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 18).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 13).is_ok());
    
        // transition into InitiativePass
        assert!(game.start_combat_rounds().is_ok());

    }

    #[test]
    pub fn if_game_is_in_precombat_phase_with_no_combatants_start_initiative_phase_generates_no_combatants()
    {
        let lef = Character::new_npc(Metatypes::Elf, String::from("Lef"));

        let mut game = Game::new();

        game.add_cast_member(lef);

        let result = game.start_initiative_phase();

        assert!(result.is_err());

        match result
        {
            Ok(_) => {panic!("This should not have succeeded.")},
            Err(err) => 
            {
                match err.kind
                {
                    crate::tracker::game::ErrorKind::UnknownCastId => {},
                    _ => {panic!("Test expected different error type (UnknownCastId)");}
                }
            },
        }

        assert_eq!(game.current_state(), String::from("PreCombat"));
    }

    #[test]
    pub fn adding_initiative_roll_before_entering_initiative_phase_results_in_invalid_state_action()
    {
        let lef = Character::new_npc(Metatypes::Elf, String::from("Lef"));
        let mut game = Game::new();

        let lef_id = game.add_cast_member(lef);
        assert!(game.add_combatant(lef_id).is_ok());

        let result = game.accept_initiative_roll(lef_id, 4);

        assert!(result.is_err());
        match result
        {
            Ok(_) => {panic!("This should not have succeeded.")},
            Err(err) => 
            {
                match err.kind
                {
                    crate::tracker::game::ErrorKind::InvalidStateAction => {},
                    _ => {panic!("Test expected different error type (UnknownCastId)");}
                }
            },
        }
    }

    #[test]
    pub fn adding_an_initiative_for_a_combatant_in_initiative_phase_will_return_ok()
    {
        init();

        let zorc = build_orc();
        let dorf = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dorf);
        assert!(game.start_initiative_phase().is_ok());

        let result = game.accept_initiative_roll(*ids.get(0).unwrap(), 4);

        assert!(result.is_ok());
    }

    #[test]
    pub fn adding_multiple_initiatives_to_real_combatants_will_produce_ok_for_each()
    {
        init();


        let zorc = build_orc();
        let dorf = build_dwarf();
        

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dorf);
        assert!(game.start_initiative_phase().is_ok());

        let result = game.accept_initiative_roll(*ids.get(0).unwrap(), 2);
        assert!(result.is_ok());
        let result = game.accept_initiative_roll(*ids.get(1).unwrap(), 13);
        assert!(result.is_ok());
    }

    #[test]
    pub fn add_initiative_for_character_id_not_in_cast_produces_unknown_cast_id()
    {
        init();

        let zorc = build_orc();
        let mork = build_orc();

        let mut game = Game::new();
        populate!(&mut game, zorc, mork);
        assert!(game.start_initiative_phase().is_ok());

        let result = game.accept_initiative_roll(Uuid::new_v4(), 2);
        assert!(result.is_err());
        match result
        {
            Ok(_) => {panic!("This should not have succeeded.")},
            Err(err) => 
            {
                match err.kind
                {
                    crate::tracker::game::ErrorKind::UnknownCastId => {},
                    _ => {panic!("Test expected different error type (UnknownCastId)");}
                }
            },
        }
    }

    #[test]
    pub fn as_initiative_rolls_are_added_game_initiative_state_updates()
    {
        init();

        let zorc = build_orc();
        let mork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, mork);
        assert!(game.start_initiative_phase().is_ok());

        assert!(game.are_any_initiatives_outstanding());
        assert!(game.collect_undeclared_initiatives().len() == 2);
        assert!(game.collect_undeclared_initiatives().contains(ids.get(0).unwrap()));
        assert!(game.collect_undeclared_initiatives().contains(ids.get(1).unwrap()));

        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());

        assert!(game.are_any_initiatives_outstanding());
        assert!(game.collect_undeclared_initiatives().len() == 1);
        assert!(game.collect_undeclared_initiatives().contains(ids.get(1).unwrap()));
        assert!(!game.collect_undeclared_initiatives().contains(ids.get(0).unwrap()));

        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());
        assert!(!game.are_any_initiatives_outstanding());
        assert!(game.collect_undeclared_initiatives().len() == 0);
    }

    #[test]
    pub fn when_all_initiatives_submitted_game_may_start_combat_rounds()
    {
        init();

        let zorc = build_orc();
        let mork = build_orc();
        let dork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, mork, dork);

        assert!(game.start_initiative_phase().is_ok());

        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 4).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 8).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 3).is_ok());

        let result = game.start_combat_rounds();

        assert_eq!(game.current_state(), String::from("Initiative Pass"));
        assert!(result.is_ok());

    }

    #[test]
    pub fn starting_combat_rounds_before_all_initiatives_submitted_produces_invalid_state_action()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();
        let dork = build_dwarf();

        let mut game = Game::new ();
        let ids = populate!(&mut game, zorc, dork, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 1).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 15).is_ok());

        let result = game.start_combat_rounds();

        assert_eq!(game.current_state(), String::from("Initiative Rolls"));
        assert!(result.is_err());

        match result
        {
            Ok(_) => {panic!("This should not have succeeded.")},
            Err(err) => 
            {
                match err.kind
                {
                    crate::tracker::game::ErrorKind::InvalidStateAction => {},
                    _ => {panic!("Test expected different error type (UnknownCastId)");}
                }
            },
        }
    }

    #[test]
    pub fn attempting_to_move_to_next_initiative_pass_fails_with_unresolved_combatant_if_there_are_unresolved_players_in_init_sequence()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();
        let dork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dork, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 20).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 33).is_ok());
        assert!(game.start_combat_rounds().is_ok());
        
        // Attempting to advance to the next initiative pass should result in failure.
        let result = game.next_initiative_pass();
        assert!(result.is_err());
        match result
        {
            Ok(_) => {panic!("This should not have succeeded.")},
            Err(err) => 
            {
                match err.kind
                {
                    crate::tracker::game::ErrorKind::UnresolvedCombatant => {},
                    _ => {panic!("Test expected different error type (UnknownCastId)");}
                }
            },
        }

    }

    #[test]
    pub fn advancing_to_next_initiative_pass_if_no_character_has_another_pass_results_in_error_end_of_initiative_pass()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();
        let dork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dork, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 20).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 33).is_ok());
        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_ok());
        assert!(game.take_action(*ids.get(0).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_ok());
        assert!(game.take_action(*ids.get(2).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_err());

        // assert!(game.next_initiative_pass().is_err());
        match game.next_initiative_pass()
        {
            Ok(_) => {panic!("This should not have succeeded.")},
            Err(err) => 
            {
                match err.kind
                {
                    crate::tracker::game::ErrorKind::EndOfInitiativePass => {},
                    _ => {panic!("Test expected different error type (UnknownCastId)");}
                }
            },
        }

    }

    #[test]
    pub fn advancing_to_next_pass_before_begin_combat_round_generates_unresolved_combatant()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();
        let dork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dork, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 20).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 33).is_ok());
        
        // assert!(game.next_initiative_pass().is_err());
        match game.next_initiative_pass()
        {
            Ok(_) => {panic!("This should not have succeeded.")},
            Err(err) => 
            {
                match err.kind
                {
                    crate::tracker::game::ErrorKind::InvalidStateAction => {},
                    _ => {panic!("Test expected different error type (UnknownCastId)");}
                }
            },
        }
        
    }

    #[test]
    pub fn initiative_may_advance_when_all_combatants_at_current_initiative_have_resolved()
    {
        let zorc = build_orc();
        let melf = build_elf();
        let dork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dork, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 20).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 33).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_ok());
        assert!(game.take_action(*ids.get(0).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_ok());
        assert!(game.take_action(*ids.get(2).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_err());
        
    }

    #[test]
    pub fn advancing_initiative_before_starting_initiative_character_has_acted_generates_unresolved_combatant()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();
        let dork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dork, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 20).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 33).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        // assert!(game.next_initiative().is_err());
        match game.advance_round()
        {
            Ok(_) => {panic!("This should not have succeeded.")},
            Err(err) => 
            {
                match err.kind
                {
                    crate::tracker::game::ErrorKind::UnresolvedCombatant => {},
                    _ => {panic!("Test expected different error type (UnknownCastId)");}
                }
            },
        }

    }

    // // TODO: Add tests to ensure advance_initiative_pass succeeds when at least one character or event occurs on the next pass.
    // // Currently cannot do that because there's no Game infrastructure to calculate or influence the pass count for any character,
    // // or add some on-next-pass event.

    #[test]
    pub fn a_resolving_character_can_take_two_simple_actions_but_not_one_simple_one_complex_nor_three_simple()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();
        let dork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dork, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 20).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 33).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Simple).is_ok());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_err());
        assert!(game.advance_round().is_err());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Simple).is_ok());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Simple).is_err());
        assert!(game.advance_round().is_ok());
    }

    #[test]
    pub fn a_resolving_character_may_take_one_complex_action_but_not_one_complex_one_single_or_two_complex()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();
        let dork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dork, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 20).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 33).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_ok());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Simple).is_err());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_err());
        assert!(game.advance_round().is_ok());
    }

    #[test]
    pub fn a_resolving_character_may_take_their_free_action_alongside_their_simple_actions()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();
        let dork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dork, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 20).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 33).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Free).is_ok());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Simple).is_ok());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Simple).is_ok());
        assert!(game.advance_round().is_ok());
    }

    #[test]
    pub fn a_resolving_character_may_take_their_free_action_alongside_their_complex_action()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();
        let dork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dork, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 20).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 33).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Free).is_ok());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_ok());
    }

    #[test]
    pub fn any_character_may_take_their_free_action_on_any_turn()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();
        let dork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dork, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 20).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 33).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(2).unwrap(), ActionType::Free).is_ok());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Simple).is_ok());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Simple).is_ok());
        assert!(game.advance_round().is_ok());
    }

    #[test]
    pub fn a_character_not_resolving_may_not_take_a_simple_action()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();
        let dork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dork, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 20).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 33).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Free).is_ok());
        assert!(game.take_action(*ids.get(2).unwrap(), ActionType::Simple).is_err());
        assert!(game.take_action(*ids.get(0).unwrap(), ActionType::Simple).is_err());
        assert!(game.advance_round().is_err());
    }

    #[test]
    pub fn a_character_not_resolving_may_not_take_a_complex_action()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();
        let dork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dork, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 20).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 33).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Free).is_ok());
        assert!(game.take_action(*ids.get(2).unwrap(), ActionType::Complex).is_err());
        assert!(game.advance_round().is_err());
    }

    #[test]
    pub fn multiple_characters_may_have_same_initiative_and_may_act_together()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();
        let dork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dork, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 13).is_ok());

        assert!(game.start_combat_rounds().is_ok());

        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Simple).is_ok());
        assert!(game.take_action(*ids.get(0).unwrap(), ActionType::Complex).is_ok());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Simple).is_ok());

    }

    #[test]
    pub fn advancing_the_initiative_round_before_all_active_characters_have_resolved_will_generate_unresolved_combatant()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();
        let dork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dork, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 13).is_ok());

        assert!(game.start_combat_rounds().is_ok());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Simple).is_ok());
        assert!(game.take_action(*ids.get(0).unwrap(), ActionType::Complex).is_ok());
        
        match game.advance_round()
        {
            Ok(_) => panic!("This should have failed."),
            Err(err) => match err.kind
            {
                crate::tracker::game::ErrorKind::UnresolvedCombatant => {},
                _ => {panic!("Advancing round without resolving all actions should generate UnresolvedCombatant type error.")}
            },
        }
    }

    #[test]
    pub fn the_initiative_round_may_be_advanced_only_after_all_current_players_have_fully_resolved()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();
        let dork = build_dwarf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, dork, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(2).unwrap(), 13).is_ok());

        assert!(game.start_combat_rounds().is_ok());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_ok());
        assert!(game.take_action(*ids.get(0).unwrap(), ActionType::Complex).is_ok());

        assert!(game.advance_round().is_ok());

        assert!(game.take_action(*ids.get(2).unwrap(), ActionType::Simple).is_ok());
    }

    // TODO: Tests for Initiative Passes once passes are in.

    #[test]
    pub fn advancing_initiative_after_all_events_have_been_processed_results_in_end_of_initiative_error()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 23).is_ok());

        assert!(game.start_combat_rounds().is_ok());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_ok());
        assert!(game.take_action(*ids.get(0).unwrap(), ActionType::Complex).is_ok());

        match game.advance_round()
        {
            Ok(_) => panic!("No new events on deck to process this round!"),
            Err(err) => {
                match err.kind
                {
                    crate::tracker::game::ErrorKind::EndOfInitiative => {},
                    _ => {panic!("Should indicate we have hit the end of the round with EndOfInitiative")}
                }
            },
        }
    }

    #[test]
    pub fn when_all_events_have_resolved_combat_may_move_back_to_initiative_phase()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 14).is_ok());

        assert!(game.start_combat_rounds().is_ok());
        assert!(game.take_action(*ids.get(0).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_ok());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_ok());

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 12).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 23).is_ok());

        assert!(game.start_combat_rounds().is_ok());
        assert!(game.take_action(*ids.get(0).unwrap(), ActionType::Complex).is_err());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_ok());
        assert!(game.take_action(*ids.get(0).unwrap(), ActionType::Complex).is_ok());
    }

    #[test]
    pub fn calling_start_initiative_phase_before_all_events_resolve_generates_unresolved_combatant()
    {
        init();

        let zorc = build_orc();
        let melf = build_elf();

        let mut game = Game::new();
        let ids = populate!(&mut game, zorc, melf);

        assert!(game.start_initiative_phase().is_ok());
        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 23).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 12).is_ok());

        assert!(game.start_combat_rounds().is_ok());
        assert!(game.take_action(*ids.get(0).unwrap(), ActionType::Complex).is_ok());
        assert!(game.advance_round().is_ok());

        match game.start_initiative_phase()
        {
            Ok(_) => {panic!("Attempting to start the initiative phase before all characters in the last turn resolve should have failed.")},
            Err(err) => match err.kind
            {
                crate::tracker::game::ErrorKind::UnresolvedCombatant => {},
                _ => {panic!("Attempting to start the initiative phase before all characters in the last turn resolve should have generated UnresolvedCombatant")}
            },
        }

        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_ok());

        assert!(game.start_initiative_phase().is_ok());

        assert!(game.accept_initiative_roll(*ids.get(0).unwrap(), 13).is_ok());
        assert!(game.accept_initiative_roll(*ids.get(1).unwrap(), 16).is_ok());
        assert!(game.start_combat_rounds().is_ok());
        assert!(game.take_action(*ids.get(1).unwrap(), ActionType::Complex).is_ok());

        match game.start_initiative_phase()
        {
            Ok(_) => {panic!("Attempting to start the initiative phase before all on-deck characters resolve should have generated UnresolvedCombatant")},
            Err(err) => match err.kind
            {
                crate::tracker::game::ErrorKind::UnresolvedCombatant => {},
                _ => {panic!("Attempting to start the initiative phase before all on-deck characters resolve should have generated UnresolvedCombatant.")}
            }
        }
    }

}