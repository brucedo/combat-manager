use std::collections::{HashMap, HashSet};

use rocket::serde::json::serde_json::map::Entry;
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
            std::collections::hash_map::Entry::Occupied(_) => Err(()),
            std::collections::hash_map::Entry::Vacant(vacant) => 
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
            std::collections::hash_map::Entry::Occupied(player_dir) => 
            {
                Some(player_dir.get().player_sender.clone())
            },
            std::collections::hash_map::Entry::Vacant(_) => 
            {
                None
            },
        }
    }
}

#[cfg(test)]
pub mod tests
{
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
    pub fn players_who_is_registered_may_can_join_a_game()
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
    pub async fn a_players_sending_channel_endpoint_may_be_retrieved_with_the_player_id()
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
}