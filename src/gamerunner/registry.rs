use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry as MapEntry;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::tracker::game::Game;

use super::GameUpdates;

pub struct PlayerDirectoryEntry
{
    pub player_id: Uuid,
    pub player_games: HashSet<Uuid>,
    pub player_characters: HashSet<Uuid>,
    pub player_sender: Sender<GameUpdates>
}

pub struct GameDirectoryEntry
{
    pub game: Game,
    pub players: HashSet<Uuid>,
}

pub struct GameRegistry
{
    games: HashMap<Uuid, GameDirectoryEntry>,
    players: HashMap<Uuid, PlayerDirectoryEntry>
}

impl <'a> GameRegistry
{

    pub fn new() -> GameRegistry
    {
        GameRegistry { games: HashMap::new(), players: HashMap::new() }
    }

    pub fn new_game(&'a mut self, id: Uuid, game: Game)
    {
        let directory_entry = GameDirectoryEntry{ game, players: HashSet::new() };
        self.games.insert(id, directory_entry);
    }

    pub fn get_mut_game(&'a mut self, id: Uuid) -> Option<&'a mut Game>
    {
        if let Some(dir_entry) = self.games.get_mut(&id)
        {
            return Some(&mut dir_entry.game);
        }
        else
        {
            return None;
        }
    }

    pub fn register_player(&mut self, player_id: Uuid, player_comm_channel: Sender<GameUpdates>) -> Result<(), ()>
    {
        match self.players.entry(player_id)
        {
            MapEntry::Occupied(_) => Err(()),
            MapEntry::Vacant(vacant) => 
            {
                vacant.insert(PlayerDirectoryEntry { player_id, player_games: HashSet::new(), player_characters: HashSet::new(), player_sender: player_comm_channel });
                Ok(())
            },
        }
    }

    pub fn join_game(&mut self, player_id: Uuid, game_id: Uuid) -> Result<(), ()>
    {
        if self.games.contains_key(&game_id) && self.players.contains_key(&player_id)
        {
            let game_dir = self.games.get_mut(&game_id).unwrap();
            let player_dir = self.players.get_mut(&player_id).unwrap();

            game_dir.players.insert(player_id);
            player_dir.player_games.insert(game_id);

            Ok(())
        }
        else
        {
            Err(())
        }
    }

    pub fn get_player_sender(&mut self, player_id: Uuid) -> Option<Sender<GameUpdates>>
    {
        match self.players.entry(player_id)
        {
            MapEntry::Occupied(player_dir) => 
            {
                Some(player_dir.get().player_sender.clone())
            },
            MapEntry::Vacant(_) => 
            {
                None
            },
        }
    }

    pub fn game_has_player(&self, game_id: Uuid, player_id: Uuid) -> bool
    {
        self.games.contains_key(&game_id) && self.games.get(&game_id).unwrap().players.contains(&player_id)
    }

    pub fn player_in_game(&self, player_id: Uuid, game_id: Uuid) -> bool
    {
        self.players.contains_key(&player_id) && self.players.get(&player_id).unwrap().player_games.contains(&game_id)
    }

    pub fn games_by_player(&self, player_id: Uuid) -> Option<&HashSet<Uuid>>
    {
        if self.players.contains_key(&player_id)
        {
            Some(&self.players.get(&player_id).unwrap().player_games)
        }
        else
        {
            None
        }
    }

    pub fn players_by_game(&mut self, game_id: Uuid) -> Option<&HashSet<Uuid>>
    {
        if self.games.contains_key(&game_id)
        {
            Some(&self.games.get(&game_id).unwrap().players)
        }
        else
        {
            None
        }
    }

    pub fn leave_game(&mut self, player_id: Uuid, game_id: Uuid) -> Result<(), ()>
    {
        match (self.games.entry(game_id), self.players.entry(player_id))
        {
            (MapEntry::Occupied(mut game_entry), MapEntry::Occupied(mut player_entry)) =>
            {
                let removed_player = game_entry.get_mut().players.remove(&player_id);
                let removed_game = player_entry.get_mut().player_games.remove(&game_id);
                if !removed_player || !removed_game
                {
                    return Err(());
                }
                else
                {
                    return Ok(());
                }
            },
            _ => {Err(())}
        }
    }

    pub fn enumerate_games(&self) -> HashSet<Uuid>
    {
        let mut result = HashSet::new();

        self.games.keys().for_each(|f| {result.insert(*f);});

        return result;
    }

    pub fn enumerate_players(&self) -> HashSet<Uuid>
    {
        let mut result = HashSet::new();

        self.players.keys().for_each(|f|{result.insert(*f);});

        return result;
    }

    pub fn delete_game(&mut self, game_id: Uuid) -> Result<(), ()>
    {
        if let Some(game) = self.games.remove(&game_id)
        {
            let mut players = game.players;

            for player in players.drain()
            {
                match self.players.entry(player)
                {
                    MapEntry::Occupied(mut player_entry) => {
                        player_entry.get_mut().player_games.remove(&game_id);
                    },
                    MapEntry::Vacant(_) => {}
                }
            }

            Ok(())
        }
        else
        {
            return Err(())
        }
    }

    pub fn unregister_player(&mut self, player_id: Uuid) -> Result<(), ()>
    {
        if let Some(player) = self.players.remove(&player_id)
        {
            let mut game_ids = player.player_games;

            for game_id in game_ids.drain()
            {
                match self.games.entry(game_id)
                {
                    MapEntry::Occupied(mut game_entry) => 
                    {
                        game_entry.get_mut().players.remove(&player_id);
                    },
                    MapEntry::Vacant(_) => {}
                }
            }

            Ok(())
        }
        else 
        {
            Err(())
        }
    }

    pub fn is_registered(&self, player_id: Uuid) -> bool
    {
        self.players.contains_key(&player_id)
    }
}

#[cfg(test)]
pub mod tests
{
    use std::collections::HashSet;

    use tokio::sync::mpsc::channel;
    use uuid::Uuid;

    use crate::{tracker::game::Game, gamerunner::GameUpdates};

    use super::GameRegistry;

    pub fn init()
    {
        let _ = env_logger::builder().is_test(true).try_init();
    }


    #[test]
    pub fn if_a_registry_holds_a_valid_game_for_a_given_id_then_get_mut_game_return_a_mutable_ref_wrapped_in_ok()
    {
        let mut registry = GameRegistry::new();

        let game: Game = Game::new();
        let id = Uuid::new_v4();

        registry.new_game(id, game);

        assert!(registry.get_mut_game(id).is_some());
    }

    #[test]
    pub fn if_a_registry_does_not_hold_a_valid_game_for_some_id_then_get_mut_game_will_return_none()
    {
        let mut registry = GameRegistry::new();

        let id = Uuid::new_v4();

        assert!(registry.get_mut_game(id).is_none());
    }

    #[test]
    pub fn a_player_who_registers_will_receive_an_ok_if_registration_completed()
    {
        let mut registry = GameRegistry::new();

        let player_id = Uuid::new_v4();
        let (sender, _) = channel(32);

        assert!(registry.register_player(player_id, sender).is_ok())
    }

    #[test]
    pub fn a_duplicate_player_id_may_not_be_used_to_register_a_new_player()
    {
        let mut registry = GameRegistry::new();

        let player_id = Uuid::new_v4();
        let (sender, _) = channel(32);
        
        registry.register_player(player_id, sender.clone());
        assert!(registry.register_player(player_id, sender).is_err());
    }

    #[test]
    pub fn players_who_is_registered_may_join_a_game()
    {
        let mut registry = GameRegistry::new();

        let player_id = Uuid::new_v4();
        let (sender, _) = channel(32);
        let game_id = Uuid::new_v4();
        let game = Game::new();

        registry.new_game(game_id, game);
        registry.register_player(player_id, sender);

        assert!(registry.join_game(player_id, game_id).is_ok());
    }

    #[test]
    pub fn a_player_who_is_not_registered_will_generate_err_when_joining_a_game()
    {
        let mut registry = GameRegistry::new();

        let player_id = Uuid::new_v4();
        let game_id = Uuid::new_v4();
        let game = Game::new();

        registry.new_game(game_id, game);

        assert!(registry.join_game(player_id, game_id).is_err());
    }

    #[tokio::test]
    pub async fn a_players_sending_channel_may_be_retrieved_with_the_player_id()
    {
        let mut registry = GameRegistry::new();
        let player_id = Uuid::new_v4();
        let (sender, mut receiver) = channel(32);
        let game_id = Uuid::new_v4();
        let game = Game::new();

        registry.new_game(game_id, game);
        registry.register_player(player_id, sender);

        let player_comms = registry.get_player_sender(player_id).unwrap();
        
        player_comms.send(GameUpdates{}).await;

        assert!(receiver.recv().await.is_some());
    }

    #[test]
    pub fn a_player_may_enumerate_the_games_they_are_in()
    {
        init();
        let mut registry = GameRegistry::new();
        let game_1 = Uuid::new_v4();
        let game_2 = Uuid::new_v4();
        let game_3 = Uuid::new_v4();

        let (mut sender, _) = channel(32);
        let player_1 = Uuid::new_v4();


        registry.new_game(game_1, Game::new());
        registry.new_game(game_2, Game::new());
        registry.new_game(game_3, Game::new());
        registry.register_player(player_1, sender);

        registry.join_game(player_1, game_1);
        registry.join_game(player_1, game_2);
        registry.join_game(player_1, game_3);

        let games: &HashSet<Uuid> = registry.games_by_player(player_1).unwrap();

        assert_eq!(3, games.len());
        assert!(games.contains(&game_1));
        assert!(games.contains(&game_2));
        assert!(games.contains(&game_3));
    }

    #[test]
    pub fn a_full_list_of_games_may_be_retrieved_with_enumerate_games()
    {
        init();

        let mut registry = GameRegistry::new();

        let game_1 = Uuid::new_v4();
        let game_2 = Uuid::new_v4();
        let game_3 = Uuid::new_v4();

        registry.new_game(game_1, Game::new());
        registry.new_game(game_2, Game::new());
        registry.new_game(game_3, Game::new());

        let games = registry.enumerate_games();

        assert_eq!(3, games.len());
        assert!(games.contains(&game_1));
        assert!(games.contains(&game_2));
        assert!(games.contains(&game_3));
    }

    #[test]
    pub fn if_no_games_have_been_created_then_enumerate_games_returns_an_empty_set()
    {
        init();

        let mut registry = GameRegistry::new();

        let games = registry.enumerate_games();

        assert_eq!(0, games.len());
    }

    #[test]
    pub fn a_full_list_of_registered_players_may_be_retrieved_with_enumerate_player()
    {
        init();
        let mut registry = GameRegistry::new();
        let player_1 = Uuid::new_v4();
        let (sender_1, _) = channel(32);
        let player_2 = Uuid::new_v4();
        let (sender_2, _) = channel(32);

        registry.register_player(player_1, sender_1);
        registry.register_player(player_2, sender_2);

        let registered_players = registry.enumerate_players();

        assert!(registered_players.len() == 2);
        assert!(registered_players.contains(&player_1));
        assert!(registered_players.contains(&player_2));
    }

    #[test]
    pub fn if_no_players_have_registered_enumerate_players_returns_an_empty_set()
    {
        init();
        let mut registry = GameRegistry::new();

        let registered_players = registry.enumerate_players();

        assert_eq!(0, registered_players.len());
    }

    #[test]
    pub fn if_the_argument_to_games_by_player_does_not_resolve_to_a_registered_player_none_is_returned()
    {
        init();
        let mut registry = GameRegistry::new();
        let game_1 = Uuid::new_v4();

        let (mut sender, _) = channel(32);
        let player_1 = Uuid::new_v4();

        registry.new_game(game_1, Game::new());
        registry.register_player(player_1, sender);

        assert!(registry.games_by_player(Uuid::new_v4()).is_none());

    }

    #[test]
    pub fn if_a_player_is_not_in_any_games_then_games_by_player_will_return_an_empty_set()
    {
        init();
        let mut registry = GameRegistry::new();
        let game_1 = Uuid::new_v4();
        registry.new_game(game_1, Game::new());

        let player_1 = Uuid::new_v4();
        let (mut sender, _) = channel(32);
        registry.register_player(player_1, sender);

        let game = registry.games_by_player(player_1).unwrap();

        assert_eq!(0, game.len());
    }

    #[test]
    pub fn a_game_may_enumerate_the_players_who_have_joined_it()
    {
        let mut registry = GameRegistry::new();
        let game_1 = Uuid::new_v4();

        let player_1 = Uuid::new_v4();
        let (sender_1, _) = channel(32);
        let player_2 = Uuid::new_v4();
        let (sender_2, _) = channel(32);
        let player_3 = Uuid::new_v4();
        let (sender_3, _) = channel(32);

        registry.register_player(player_1, sender_1);
        registry.register_player(player_2, sender_2);
        registry.register_player(player_3, sender_3);

        registry.new_game(game_1, Game::new());

        registry.join_game(player_1, game_1);
        registry.join_game(player_2, game_1);
        registry.join_game(player_3, game_1);

        let players: &HashSet<Uuid> = registry.players_by_game(game_1).unwrap();

        assert_eq!(3, players.len());
        assert!(players.contains(&player_1));
        assert!(players.contains(&player_2));
        assert!(players.contains(&player_3));
    }

    #[test]
    pub fn if_a_game_exists_but_has_no_players_a_reference_to_an_empty_set_is_returned()
    {
        init();

        let mut registry = GameRegistry::new();

        let game_1 = Uuid::new_v4();
        let player_1 = Uuid::new_v4();
        let (sender_1, _) = channel(32);

        registry.new_game(game_1, Game::new());
        registry.register_player(player_1, sender_1);
        
        let players = registry.players_by_game(game_1);
        assert!(players.is_some());
        assert!(players.unwrap().len() == 0);
    }


    #[test]
    pub fn if_the_argument_to_players_by_game_does_not_resolve_to_a_real_game_then_none_is_returned()
    {
        init();
        let mut registry = GameRegistry::new();
        
        let game_1 = Uuid::new_v4();

        let player_1 = Uuid::new_v4();
        let (sender, _) = channel(32);

        registry.new_game(game_1, Game::new());
        registry.register_player(player_1, sender);

        registry.join_game(player_1, game_1);

        assert!(registry.players_by_game(Uuid::new_v4()).is_none());
    }

    #[test]
    pub fn if_a_player_leaves_a_game_both_the_player_and_the_game_records_update_to_remove_the_player()
    {
        init();

        let mut registry = GameRegistry::new();
        let player_1 = Uuid::new_v4();
        let player_2 = Uuid::new_v4();
        let (mut sender, _) = channel(32);
        let game_id = Uuid::new_v4();
        let game = Game::new();

        registry.new_game(game_id, game);
        registry.register_player(player_1, sender);
        (sender, _) = channel(32);
        registry.register_player(player_2, sender);

        registry.join_game(player_1, game_id);
        registry.join_game(player_2, game_id);

        assert!(registry.leave_game(player_1, game_id).is_ok());
        assert!(!registry.game_has_player(game_id, player_1));
        assert!(!registry.player_in_game(player_1, game_id));
    }

    #[test]
    pub fn if_a_player_leaves_a_game_they_are_not_a_member_of_leave_game_returns_err()
    {
        init();

        let mut registry = GameRegistry::new();
        let player_1 = Uuid::new_v4();
        let game_id = Uuid::new_v4();
        let game = Game::new();

        registry.new_game(game_id, game);

        assert!(registry.leave_game(player_1, game_id).is_err());
    }

    #[test]
    pub fn when_a_game_is_deleted_all_players_are_updated_to_remove_that_game_from_their_lists()
    {
        init();

        let mut registry = GameRegistry::new();
        let player_1 = Uuid::new_v4();
        let (sender_1, _) = channel(32);
        let player_2 = Uuid::new_v4();
        let (sender_2, _) = channel(32);
        let player_3 = Uuid::new_v4();
        let (sender_3, _) = channel(32);
        
        let game_id = Uuid::new_v4();

        registry.new_game(game_id, Game::new());
        registry.register_player(player_1, sender_1);
        registry.register_player(player_2, sender_2);
        registry.register_player(player_3, sender_3);

        registry.join_game(player_1, game_id);
        registry.join_game(player_2, game_id);
        registry.join_game(player_3, game_id);

        registry.delete_game(game_id);

        assert!(registry.players_by_game(game_id).is_none());
        assert!(!registry.player_in_game(player_1, game_id));
        assert!(!registry.player_in_game(player_2, game_id));
        assert!(!registry.player_in_game(player_3, game_id));
        
    }

    pub fn when_delete_game_is_passed_an_id_that_does_not_represent_a_real_game_err_is_returned()
    {
        init();

        let mut registry = GameRegistry::new();

        assert!(registry.delete_game(Uuid::new_v4()).is_err());
    }

    pub fn when_a_player_unregisters_they_are_removed_from_all_games_they_are_part_of()
    {
        init();
        let  mut registry = GameRegistry::new();

        let player_1 = Uuid::new_v4();
        let (sender_1, _) = channel(32);
        let player_2 = Uuid::new_v4();
        let (sender_2, _) = channel(32);

        let game_1 = Uuid::new_v4();
        let game_2 = Uuid::new_v4();

        registry.new_game(game_1, Game::new());
        registry.new_game(game_2, Game::new());
        registry.register_player(player_1, sender_1);
        registry.register_player(player_2, sender_2);

        registry.join_game(player_1, game_1);
        registry.join_game(player_2, game_1);
        registry.join_game(player_1, game_2);

        registry.unregister_player(player_1);

        let players = registry.players_by_game(game_1);
        assert!(players.is_some() && players.unwrap().contains(&player_2));
        assert!(!players.unwrap().contains(&player_1));

        let players = registry.players_by_game(game_2);
        assert!(players.is_some());
        assert!(players.unwrap().len() == 0);
    }

    #[test]
    pub fn when_an_unrecognized_player_id_is_handed_to_unregister_Err_is_returned()
    {
        init();

        let mut registry = GameRegistry::new();

        let player_1 = Uuid::new_v4();
        let (sender_1, _) = channel(32);
        let player_2 = Uuid::new_v4();
        let (sender_2, _) = channel(32);

        let game_1 = Uuid::new_v4();
        let game_2 = Uuid::new_v4();

        registry.new_game(game_1, Game::new());
        registry.new_game(game_2, Game::new());
        registry.register_player(player_1, sender_1);
        registry.register_player(player_2, sender_2);

        let result = registry.unregister_player(Uuid::new_v4());

        assert!(result.is_err());
    }

    #[test]
    pub fn is_registered_will_return_true_if_a_given_id_is_registered_with_the_directory()
    {
        init();

        let mut registry = GameRegistry::new();

        let player_1 = Uuid::new_v4();
        let (sender_1, _) = channel(32);

        let game_1 = Uuid::new_v4();

        registry.register_player(player_1, sender_1);
        registry.new_game(game_1, Game::new());

        registry.join_game(player_1, game_1);

        assert!(registry.is_registered(player_1));
    }

    #[test]
    pub fn is_registered_will_return_false_if_a_given_id_is_not_registered_with_the_directory()
    {
        init();

        let mut registry = GameRegistry::new();

        assert!(!registry.is_registered(Uuid::new_v4()));
    }

    #[test]
    pub fn is_registered_returns_true_regardless_of_whether_a_player_has_joined_any_game()
    {
        init();

        let mut registry = GameRegistry::new();

        let player_1 = Uuid::new_v4();
        let (sender_1, _) = channel(32);
        let player_2 = Uuid::new_v4();
        let (sender_2, _) = channel(32);

        let game_1 = Uuid::new_v4();

        registry.new_game(game_1, Game::new());
        registry.register_player(player_1, sender_1);
        registry.register_player(player_2, sender_2);

        registry.join_game(player_1, game_1);

        assert!(registry.is_registered(player_1));
        assert!(registry.is_registered(player_2));
    }

}