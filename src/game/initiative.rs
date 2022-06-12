use std::collections::{BinaryHeap, BTreeMap};
use std::ops::Bound::Included;
use env_logger::init;
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

    pub fn get_ordered_inits(&mut self) -> Vec::<(i8, Uuid)>
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
    pub fn test_add_single()
    {
        let mut tracker = InitTracker::new(None);
        let fake_id = Uuid::new_v4();

        tracker.add_new_event(fake_id, 4, 1, 0, 0);

        assert_eq!(tracker.initiatives.len(), 1);
    }

    #[test]
    pub fn test_on_next_pass()
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
    pub fn test_one_shot_next_pass()
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
    pub fn test_get_inits_random()
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
    pub fn test_inits_match_ids()
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
    pub fn test_duplicate_inits_match_ids()
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
    pub fn test_begin_pass()
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
    pub fn test_begin_pass_increments_pass()
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
    pub fn test_begin_pass_on_empty()
    {
        init();
        
        let mut tracker = InitTracker::new(None);

        assert_eq!(tracker.begin_new_pass(), PassState::AllDone);
        assert_eq!(tracker.current_pass, 0);
        assert_eq!(tracker.begin_new_pass(), PassState::AllDone);
        assert_eq!(tracker.current_pass, 0);
    }

    #[test]
    pub fn test_begin_pass_with_overflow()
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
    pub fn test_next()
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
    pub fn test_empty_next()
    {
        init();

        let mut tracker = InitTracker::new(None);
        
        assert_eq!(PassState::PassDone, tracker.next());
    }

    #[test]
    pub fn test_multi_pass()
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
    pub fn test_reset()
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
    pub fn test_set_astral()
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
    pub fn test_set_astral_multi_pass()
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
    pub fn test_fail_set_astral()
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
    pub fn test_set_matrix()
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
    pub fn test_set_matrix_multi_pass()
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
    pub fn test_fail_set_matrix()
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
    pub fn test_advance_matrix_pass()
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
    pub fn test_advance_astral_pass()
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