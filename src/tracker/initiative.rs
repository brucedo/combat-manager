use uuid::Uuid;


pub struct InitTracker {
    // The primary initiative tracker: all initiatives are inserted into the heap and popped on demand.
    initiatives: Vec<Initiative>,

    current_pass: usize, // pass tracker,
    // post-turn storage.  An Initiative that is popped from the heap but can still act on a subsequent turn is held here.
    // During the pass turn-over, it will be moved back into initiatives for the next go round.
    overflow: Vec<Initiative>,
}

struct Initiative {
    pub id: Uuid,
    pub initiative: i8,
    pub in_astral_space: bool,
    pub astral_passes: usize,
    pub in_matrix: bool,
    pub matrix_passes: usize,
    pub passes: usize,
}

#[derive(PartialEq, Debug)]
pub enum PassState {
    AcceptedRequest,
    DeniedRequest(String), // Not in use yet - reserved for later functionality that may need to reject a call for some reason.
    Ready,
    PassDone,
    AllDone,
    Next((Uuid, i8)),
    UnknownId(Uuid),
}

impl InitTracker {

    pub fn current_pass(&self) -> usize
    {
        *(&self.current_pass)
    }

    pub fn new(size_hint: Option<usize>) -> InitTracker {
        match size_hint
        {
            Some(size) => 
            {
                let mut inits = Vec::new();
                inits.reserve(size);
                InitTracker { initiatives: inits, current_pass: 0, overflow: Vec::new() }
            },
            None => 
            {
                let mut inits = Vec::new();
                inits.reserve(20); // Completely arbitrary, but seems reasonable.
                InitTracker { initiatives: inits, current_pass: 0, overflow: Vec::new()}
            },
        }
        
    }

    pub fn reset(&mut self)
    {
        self.initiatives.clear();
        self.current_pass = 0;
        self.overflow.clear();
    }

    pub fn add_new_event(&mut self, id: Uuid, initiative: i8, passes: usize, astral_passes: usize, matrix_passes: usize) -> PassState
    {
        let init = Initiative{id, initiative, in_astral_space: false, astral_passes, in_matrix: false, matrix_passes, passes};
        match self.initiatives.binary_search(&init)
        {
            // It's goin' in either way - just a gotta find the right place.
            Ok(index) => self.initiatives.insert(index, init),
            Err(index) => self.initiatives.insert(index, init),
        }

        PassState::AcceptedRequest
    }

    pub fn on_next_pass(&mut self, id: Uuid, initiative: i8, passes: usize, astral_passes: usize, matrix_passes: usize) -> PassState
    {
        let init = Initiative{id, initiative, in_astral_space: false, astral_passes, in_matrix: false, matrix_passes, passes};
        self.overflow.push(init);

        PassState::AcceptedRequest
    }

    // for timed events that will happen on the next initiative pass, but only once.
    pub fn one_shot_next_pass(&mut self, id: Uuid, initiative: i8) -> PassState
    {
        let init = Initiative
        {
            id, 
            initiative, 
            in_astral_space: false, 
            passes: self.current_pass + 1,
            astral_passes: 0, 
            in_matrix: false, 
            matrix_passes: 0
        };

        self.overflow.push(init);

        PassState::AcceptedRequest
    }

    pub fn get_ordered_inits(& self) -> Vec::<(i8, Uuid)>
    {
        let mut ordering = Vec::<(i8, Uuid)>::new();
        ordering.reserve(self.initiatives.len());

        for initiative in &self.initiatives
        {
            ordering.insert(0, (initiative.initiative, initiative.id));
        }

        return ordering;
    }

    // Advance the pass tracker, feed any initiatives in the overflow back into the initiative tracker, and return ready.
    pub fn begin_new_pass(&mut self) -> PassState
    {
        for init in self.overflow.drain(0..(self.overflow.len()))
        {
            match self.initiatives.binary_search(&init)
            {
                Ok(index) => self.initiatives.insert(index, init),
                Err(index) => self.initiatives.insert(index, init)
            }
        }

                
        if self.initiatives.len() == 0
        {
            return PassState::AllDone
        }

        // No point endlessly incrementing the current pass unless there's characters in queue to actually perform actions.
        self.current_pass += 1;

        PassState::Ready
    }

    pub fn next(&mut self) -> PassState
    {
        if let Some(initiative) = self.initiatives.pop()
        {   // 
            let id = initiative.id;
            let init_value = initiative.initiative;
            if self.has_more_passes(&initiative)
            {
                self.overflow.push(initiative);
            }

            PassState::Next((id, init_value))
        }
        else 
        {
            PassState::PassDone
        }
        
    }

    pub fn next_if_match(&mut self, init: i8) -> PassState
    {
        if let Some(initiative) = self.initiatives.last()
        {
            if initiative.initiative == init
            {
                return self.next();
            }
            else 
            {
                return PassState::PassDone
            }
        }

        PassState::PassDone
    }

    fn has_more_passes(&mut self, initiative: &Initiative) -> bool
    {
        if (initiative.in_astral_space && initiative.astral_passes > self.current_pass) ||
            (initiative.in_matrix && initiative.matrix_passes > self.current_pass) ||
            (initiative.passes > self.current_pass)
        {
            return true;
        }

        false
    }

    pub fn login_matrix(&mut self, id: Uuid) -> PassState
    {
        // sadly, I'mma have to linear search.  oh well, premature optimization etc.
        for init in self.initiatives.iter_mut()
        {
            if init.id == id
            {
                init.in_matrix = true;
                return PassState::AcceptedRequest;
            }
        }

        for init in self.overflow.iter_mut()
        {
            if init.id == id
            {
                init.in_matrix = true;
                return PassState::AcceptedRequest;
            }
        }

        PassState::UnknownId(id)
    }

    pub fn logout_matrix(&mut self, id: Uuid) -> PassState
    {
        for init in self.initiatives.iter_mut()
        {
            if init.id == id
            {
                init.in_matrix = false;
                return PassState::AcceptedRequest;
            }
        }

        for init in self.overflow.iter_mut()
        {
            if init.id == id
            {
                init.in_matrix = false;
                return PassState::AcceptedRequest;
            }
        }

        PassState::UnknownId(id)
    }

    pub fn enter_astral_space(&mut self, id: Uuid) -> PassState
    {

        for init in self.initiatives.iter_mut()
        {
            if init.id == id
            {
                init.in_astral_space = true;
                return PassState::AcceptedRequest;
            }
        }

        for init in self.overflow.iter_mut()
        {
            if init.id == id
            {
                init.in_astral_space = true;
                return PassState::AcceptedRequest;
            }
        }

        PassState::UnknownId(id)
    }

    pub fn exit_astral_space(&mut self, id: Uuid) -> PassState
    {
        for init in self.initiatives.iter_mut()
        {
            if init.id == id
            {
                init.in_astral_space = false;
                return PassState::AcceptedRequest;
            }
        }

        for init in self.overflow.iter_mut()
        {
            if init.id == id
            {
                init.in_astral_space = false;
                return PassState::AcceptedRequest;
            }
        }

        PassState::UnknownId(id)
    }

    pub fn end_turn(&mut self) -> PassState
    {
        self.current_pass = 0;
        self.initiatives.clear();
        self.overflow.clear();
        
        PassState::Ready
    }

}

impl PartialEq for Initiative {

    fn eq(&self, other: &Self) -> bool {
        self.initiative == other.initiative
    }

}

impl PartialOrd for Initiative {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
        // match self.initiative.partial_cmp(&other.initiative) {
        //     Some(core::cmp::Ordering::Equal) => {}
        //     ord => return ord,
        // }
    }
}

impl Eq for Initiative {}

impl Ord for Initiative {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.initiative == other.initiative
        {
            std::cmp::Ordering::Equal
        }
        else if self.initiative > other.initiative
        {
            std::cmp::Ordering::Greater
        }
        else
        {
            std::cmp::Ordering::Less
        }
    }
}

#[cfg(test)]
mod tests
{
    use uuid::Uuid;

    use super::{InitTracker, PassState, Initiative};

    pub fn init()
    {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    pub fn adding_a_single_event_increases_event_counter_size_by_one()
    {
        let mut tracker = InitTracker::new(None);
        let fake_id = Uuid::new_v4();

        tracker.add_new_event(fake_id, 4, 1, 0, 0);

        assert_eq!(tracker.initiatives.len(), 1);
    }

    #[test]
    pub fn resetting_tracker_resets_pass_data_and_clears_all_initiatives()
    {
        let mut tracker = InitTracker::new(Some(15));

        let fake_id = Uuid::new_v4();

        tracker.add_new_event(fake_id, 4, 1, 2, 3);
        tracker.add_new_event(Uuid::new_v4(), 6, 1, 2, 3);
        tracker.begin_new_pass();

        tracker.reset();

        assert_eq!(tracker.current_pass(), 0);
        assert_eq!(tracker.get_ordered_inits().len(), 0);
    }

    #[test]
    pub fn on_next_pass_will_add_a_recurring_event_to_the_next_pass_in_the_current_initiative_round()
    {
        init();

        let mut tracker = InitTracker::new(None);
        let first_pass = Uuid::new_v4();
        let next_pass = Uuid::new_v4();

        tracker.add_new_event(first_pass, 3, 1, 3, 3);
        assert_eq!(PassState::Ready, tracker.begin_new_pass());

        tracker.on_next_pass(next_pass, 12, 2, 1, 1);
        assert_eq!(PassState::Next((first_pass, 3)), tracker.next());
        assert_eq!(PassState::PassDone, tracker.next());

        assert_eq!(PassState::Ready, tracker.begin_new_pass());
        assert_eq!(PassState::Next((next_pass, 12)), tracker.next());
    }

    #[test]
    pub fn one_shot_next_pass_will_add_an_event_to_the_next_pass_in_current_round_that_can_happen_only_once()
    {
        init();

        let mut tracker = InitTracker::new(None);
        let multi_pass = Uuid::new_v4();
        let one_shot = Uuid::new_v4();

        tracker.add_new_event(multi_pass, 3, 3, 3, 3);
        assert_eq!(PassState::Ready, tracker.begin_new_pass());

        // pass one
        assert_eq!(PassState::Next((multi_pass, 3)), tracker.next());
        // grenade!
        assert_eq!(PassState::AcceptedRequest, tracker.one_shot_next_pass(one_shot, 20));
        assert_eq!(PassState::PassDone, tracker.next());

        // pass two
        assert_eq!(PassState::Ready, tracker.begin_new_pass());
        assert_eq!(PassState::Next((one_shot, 20)), tracker.next()); // BANG
        assert_eq!(PassState::Next((multi_pass, 3)), tracker.next());
        assert_eq!(PassState::PassDone, tracker.next());

        // pass three
        assert_eq!(PassState::Ready, tracker.begin_new_pass());
        assert_eq!(PassState::Next((multi_pass, 3)), tracker.next());
        assert_eq!(PassState::PassDone, tracker.next());

        // And odne
        assert_eq!(PassState::AllDone, tracker.begin_new_pass());
        
    }

    #[test]
    pub fn get_ordered_inits_will_retrieve_all_initiatives_in_an_ascending_order()
    {
        init();

        let mut tracker = InitTracker::new(None);
        let inits:[i8; 14] = [1, 9, 9, 32, 4, 5, 22, 9, 12, 9, 8, 0, 67, 23];

        for init in inits
        {
            tracker.add_new_event(Uuid::new_v4(), init, 1, 0, 0);
        }

        let ordered_inits = tracker.get_ordered_inits();

        let mut last = i8::MAX;
        for (init, _id) in ordered_inits 
        {
            assert!(init <= last);
            last = init;
        }
    }

    #[test]
    pub fn initiatives_are_associated_with_their_event_id_even_after_sorting()
    {
        init();

        let mut tracker = InitTracker::new(None);
        let mut character_data = Vec::<(i8, Uuid)>::new();

        character_data.push((14i8, Uuid::new_v4()));
        character_data.push((12i8, Uuid::new_v4()));
        character_data.push((22i8, Uuid::new_v4()));
        character_data.push((19i8, Uuid::new_v4()));
        character_data.push((2i8, Uuid::new_v4()));
        character_data.push((100i8, Uuid::new_v4()));

        for (init, id) in &character_data
        {
            tracker.add_new_event(id.clone(), init.clone(), 1, 3, 3);
        }

        let ordered_inits = tracker.get_ordered_inits();
        for init in ordered_inits
        {
            for pair in &character_data
            {
                if init.0 == pair.0
                {
                    assert_eq!(init.1, pair.1);
                }
            }
        }
    }

    #[test]
    pub fn multiple_ids_with_the_same_initiative_do_not_get_lost_or_removed()
    {
        init();

        let mut tracker = InitTracker::new(None);
        let mut character_data = Vec::<(i8, Uuid)>::new();

        character_data.push((14i8, Uuid::new_v4()));
        character_data.push((14i8, Uuid::new_v4()));

        for (init, id) in &character_data
        {
            tracker.add_new_event(id.clone(), init.clone(), 1, 3, 3);
        }

        // whoops - can't test this yet.  Best I can do is confirm that both UUIDS are represented in the output.
        let ordered_inits = tracker.get_ordered_inits();
        for tuple in ordered_inits
        {
            assert!(character_data.contains(&tuple));
        }
    }

    #[test]
    pub fn begin_new_pass_must_be_called_to_start_the_very_first_pass_in_a_combat_round()
    {
        init();

        let mut tracker = InitTracker::new(None);
        
        for inits in 10..20
        {
            tracker.add_new_event(Uuid::new_v4(), inits, 1, 2, 3);
        }

        let state = tracker.begin_new_pass();

        assert_eq!(state, PassState::Ready);
        assert_eq!(tracker.current_pass, 1);
    }

    #[test]
    pub fn calling_begin_new_pass_repeatedly_will_advance_the_pass_number()
    {
        init();

        let mut tracker = InitTracker::new(None);

        for inits in 10..20
        {
            tracker.add_new_event(Uuid::new_v4(), inits, 1, 2, 3);
        }

        //
        
        assert_eq!(tracker.begin_new_pass(), PassState::Ready);
        assert_eq!(tracker.current_pass, 1);
        assert_eq!(tracker.begin_new_pass(), PassState::Ready);
        assert_eq!(tracker.current_pass, 2);
        assert_eq!(tracker.begin_new_pass(), PassState::Ready);
        assert_eq!(tracker.current_pass, 3);   
    }

    #[test]
    pub fn begin_new_pass_against_an_empty_initiative_list_produces_pass_state_all_done()
    {
        init();
        
        let mut tracker = InitTracker::new(None);

        assert_eq!(tracker.begin_new_pass(), PassState::AllDone);
        assert_eq!(tracker.current_pass, 0);
        assert_eq!(tracker.begin_new_pass(), PassState::AllDone);
        assert_eq!(tracker.current_pass, 0);
    }

    #[test]
    pub fn initiatives_added_to_overflow_will_migrate_into_regular_init_on_next_pass()
    {
        init();
        
        let mut tracker = InitTracker::new(None);

        for inits in 10..20
        {
            
            tracker.overflow.push(Initiative {
                id: Uuid::new_v4(),
                initiative: inits,
                in_astral_space: false,
                astral_passes: 3,
                in_matrix: false,
                matrix_passes: 4,
                passes: 5,
            })
        }

        assert_eq!(tracker.begin_new_pass(), PassState::Ready);
        assert_eq!(tracker.initiatives.len(), 10);
    }

    #[test]
    pub fn calling_next_will_advance_tracker_current_id_to_next_on_initiative_order()
    {
        init();

        let mut tracker = InitTracker::new(None);
        let mut ordered_ids = Vec::<(i8, Uuid)>::new();

        for init in 10i8..20
        {
            let id = Uuid::new_v4();
            ordered_ids.push((init, id));
            tracker.add_new_event(id, init, 1, 1, 1);
        }

        while let PassState::Next(id) = tracker.next()
        {
            let comparative_id = ordered_ids.pop().unwrap();
            assert_eq!(id.0, comparative_id.1);
        }

        // And confirm that we get PassDone now.
        assert_eq!(PassState::PassDone, tracker.next());
    }

    #[test]
    pub fn calling_next_with_an_empty_initiative_list_produces_pass_done()
    {
        init();

        let mut tracker = InitTracker::new(None);
        
        assert_eq!(PassState::PassDone, tracker.next());
    }

    #[test]
    pub fn if_pass_done_is_reached_when_any_event_has_multiple_passes_then_that_event_will_be_present_on_the_next_pass()
    {
        init();

        let mut tracker = InitTracker::new(None);
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        tracker.add_new_event(first, 23, 2, 1, 1);
        tracker.add_new_event(second, 12, 1, 1, 1);


        assert_eq!(PassState::Ready, tracker.begin_new_pass());

        assert_eq!(PassState::Next((first, 23)), tracker.next());
        assert_eq!(PassState::Next((second, 12)), tracker.next());
        assert_eq!(PassState::PassDone, tracker.next());

        assert_eq!(PassState::Ready, tracker.begin_new_pass());

        let next_events = tracker.get_ordered_inits();

        assert_eq!(1, next_events.len());
    }

    #[test]
    pub fn end_turn_resets_all_state_of_the_tracker()
    {
        init();
        
        let mut tracker = InitTracker::new(None);

        let one_shot = Uuid::new_v4();
        tracker.add_new_event(one_shot, 22, 1, 1, 1);

        assert_eq!(PassState::Ready, tracker.begin_new_pass());
        assert_eq!(PassState::Next((one_shot, 22)), tracker.next());
        assert_eq!(PassState::PassDone, tracker.next());
        assert_eq!(PassState::AllDone, tracker.begin_new_pass());

        assert_eq!(PassState::Ready, tracker.end_turn());

        tracker.add_new_event(one_shot, 22, 2, 1, 1);

        assert_eq!(PassState::Ready, tracker.begin_new_pass());
        assert_eq!(PassState::Next((one_shot, 22)), tracker.next());
        assert_eq!(PassState::PassDone, tracker.next());

        // if the reset fails, then the pass tracker won't restart.  Therefore the new event will be shortchanged - 
        // it won't get it's second pass.
        assert_eq!(PassState::Ready, tracker.begin_new_pass());
    }

    #[test]
    pub fn enter_astral_and_exit_astral_will_generate_accepted_request()
    {
        init();

        let mut tracker = InitTracker::new(None);

        let astral_id = Uuid::new_v4();
        tracker.add_new_event(astral_id, 32, 1, 2, 1);
        tracker.add_new_event(Uuid::new_v4(), 22, 2, 1, 3);

        assert_eq!(PassState::AcceptedRequest, tracker.enter_astral_space(astral_id));
        // Might as well check that we can turn it off.
        assert_eq!(PassState::AcceptedRequest, tracker.exit_astral_space(astral_id));
    }

    #[test]
    pub fn enter_and_exit_astral_may_be_called_in_middle_of_pass()
    {
        init();

        let mut tracker = InitTracker::new(None);

        let astral_id = Uuid::new_v4();
        tracker.add_new_event(astral_id, 12, 2, 2, 1);
        tracker.add_new_event(Uuid::new_v4(), 23, 1, 3, 3);

        tracker.begin_new_pass();
        tracker.next();
        tracker.next();
        
        assert_eq!(PassState::AcceptedRequest, tracker.enter_astral_space(astral_id));
        assert_eq!(PassState::AcceptedRequest, tracker.exit_astral_space(astral_id));
    }

    #[test]
    pub fn enter_and_exit_astral_when_invoked_with_an_unknown_id_generate_pass_state_unknown_id()
    {
        init();

        let mut tracker = InitTracker::new(None);

        let astral_id = Uuid::new_v4();
        tracker.add_new_event(Uuid::new_v4(), 22, 2, 1, 3);
        tracker.add_new_event(Uuid::new_v4(), 16, 1, 4, 2);

        assert_eq!(PassState::UnknownId(astral_id), tracker.enter_astral_space(astral_id));
        assert_eq!(PassState::UnknownId(astral_id), tracker.exit_astral_space(astral_id));
    }

    #[test]
    pub fn login_logout_matrix_switch_tracker_to_use_that_events_matrix_pass()
    {
        init();

        let mut tracker = InitTracker::new(None);

        let matrix_id = Uuid::new_v4();
        tracker.add_new_event(matrix_id, 32, 1, 2, 1);
        tracker.add_new_event(Uuid::new_v4(), 22, 2, 1, 3);

        assert_eq!(PassState::AcceptedRequest, tracker.login_matrix(matrix_id));
        // Might as well check that we can turn it off.
        assert_eq!(PassState::AcceptedRequest, tracker.logout_matrix(matrix_id));
    }

    #[test]
    pub fn login_logout_matrix_can_be_called_in_mid_pass()
    {
        init();

        let mut tracker = InitTracker::new(None);

        let matrix_id = Uuid::new_v4();
        tracker.add_new_event(matrix_id, 14, 2, 1, 2);
        tracker.add_new_event(Uuid::new_v4(), 23, 1, 3, 3);

        tracker.begin_new_pass();
        tracker.next();
        tracker.next();
        
        assert_eq!(PassState::AcceptedRequest, tracker.login_matrix(matrix_id));
        assert_eq!(PassState::AcceptedRequest, tracker.logout_matrix(matrix_id));
    }

    #[test]
    pub fn login_logout_matrix_will_generate_pass_state_unknow_id_when_provided_with_invalid_id()
    {
        init();

        let mut tracker = InitTracker::new(None);

        let matrix_id = Uuid::new_v4();
        tracker.add_new_event(Uuid::new_v4(), 22, 2, 1, 3);
        tracker.add_new_event(Uuid::new_v4(), 16, 1, 4, 2);

        assert_eq!(PassState::UnknownId(matrix_id), tracker.login_matrix(matrix_id));
        assert_eq!(PassState::UnknownId(matrix_id), tracker.logout_matrix(matrix_id));
    }

    #[test]
    pub fn when_an_id_is_in_matrix_mode_will_get_as_many_passes_as_specified_matrix_passes()
    {
        init();

        let mut tracker = InitTracker::new(None);

        let matrix_id = Uuid::new_v4();
        let non_matrix_id = Uuid::new_v4();

        // One pass without matrix login...
        tracker.add_new_event(matrix_id, 22, 1, 1, 3);
        tracker.add_new_event(non_matrix_id, 16, 1, 4, 1);

        assert_eq!(PassState::Ready, tracker.begin_new_pass());

        assert_eq!(PassState::Next((matrix_id, 22)), tracker.next());
        assert_eq!(PassState::Next((non_matrix_id, 16)), tracker.next());
        assert_eq!(PassState::PassDone, tracker.next());

        assert_eq!(PassState::AllDone, tracker.begin_new_pass());

        assert_eq!(PassState::Ready, tracker.end_turn());

        // And one pass with.
        tracker.add_new_event(matrix_id, 22, 1, 1, 3);
        tracker.add_new_event(non_matrix_id, 16, 1, 4, 1);
        assert_eq!(PassState::AcceptedRequest, tracker.login_matrix(matrix_id));

        // pass one
        assert_eq!(PassState::Ready, tracker.begin_new_pass());
        assert_eq!(PassState::Next((matrix_id, 22)), tracker.next());
        assert_eq!(PassState::Next((non_matrix_id, 16)), tracker.next());
        assert_eq!(PassState::PassDone, tracker.next());

        // pass two
        assert_eq!(PassState::Ready, tracker.begin_new_pass());
        assert_eq!(PassState::Next((matrix_id, 22)), tracker.next());
        assert_eq!(PassState::PassDone, tracker.next());

        // pass three
        assert_eq!(PassState::Ready, tracker.begin_new_pass());
        assert_eq!(PassState::Next((matrix_id, 22)), tracker.next());
        assert_eq!(PassState::PassDone, tracker.next());

        // and pass four should end it.
        assert_eq!(PassState::AllDone, tracker.begin_new_pass());


    }

    #[test]
    pub fn when_an_id_is_in_astral_mode_will_get_as_many_passes_as_specified_astral_passes()
    {
        init();

        let mut tracker = InitTracker::new(None);

        let non_astral_form = Uuid::new_v4();
        let astral_form = Uuid::new_v4();

        // One pass without astral flag set...
        tracker.add_new_event(non_astral_form, 22, 1, 1, 3);
        tracker.add_new_event(astral_form, 16, 1, 4, 1);

        assert_eq!(PassState::Ready, tracker.begin_new_pass());

        assert_eq!(PassState::Next((non_astral_form, 22)), tracker.next());
        assert_eq!(PassState::Next((astral_form, 16)), tracker.next());
        assert_eq!(PassState::PassDone, tracker.next());

        assert_eq!(PassState::AllDone, tracker.begin_new_pass());

        assert_eq!(PassState::Ready, tracker.end_turn());

        // And one pass with.
        tracker.add_new_event(non_astral_form, 22, 1, 1, 3);
        tracker.add_new_event(astral_form, 16, 1, 4, 1);
        tracker.enter_astral_space(astral_form);

        // pass one
        assert_eq!(PassState::Ready, tracker.begin_new_pass());
        assert_eq!(PassState::Next((non_astral_form, 22)), tracker.next());
        assert_eq!(PassState::Next((astral_form, 16)), tracker.next());
        assert_eq!(PassState::PassDone, tracker.next());

        // pass two
        assert_eq!(PassState::Ready, tracker.begin_new_pass());
        assert_eq!(PassState::Next((astral_form, 16)), tracker.next());
        assert_eq!(PassState::PassDone, tracker.next());

        // pass three
        assert_eq!(PassState::Ready, tracker.begin_new_pass());
        assert_eq!(PassState::Next((astral_form, 16)), tracker.next());
        assert_eq!(PassState::PassDone, tracker.next());

        // pass four
        assert_eq!(PassState::Ready, tracker.begin_new_pass());
        assert_eq!(PassState::Next((astral_form, 16)), tracker.next());
        assert_eq!(PassState::PassDone, tracker.next());

        // and done
        assert_eq!(PassState::AllDone, tracker.begin_new_pass());

    }

}