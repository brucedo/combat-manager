use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry as MapEntry;
use std::sync::Arc;
use log::debug;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::tracker::character::Character;
use crate::tracker::game::Game;

use super::{WhatChanged, CharacterId};

type PlayerId = Uuid;
type GameId = Uuid;

pub struct PlayerDirectoryEntry
{
    pub player_id: Uuid,
    pub player_name: String,
    pub player_games: HashSet<GameId>,
    pub player_characters: HashMap<GameId, HashSet<CharacterId>>,
    pub player_sender: Sender<Arc<WhatChanged>>
}

pub struct GameDirectoryEntry
{
    pub game: Game,
    pub name: String,
    pub gm: Uuid,
    pub players: HashSet<PlayerId>,
}

pub struct GameRegistry
{
    games: HashMap<GameId, GameDirectoryEntry>,
    players: HashMap<PlayerId, PlayerDirectoryEntry>
}

impl <'a> GameRegistry
{

    pub fn new() -> GameRegistry
    {
        GameRegistry { games: HashMap::new(), players: HashMap::new() }
    }

    pub fn new_game(&'a mut self, player_id: PlayerId, game_name: String, game_id: GameId, game: Game) -> Result<(),()>
    {
        debug!("Starting new_game()");
        if self.players.contains_key(&player_id)
        {
            debug!("Player id {} is registered as a player.", player_id);
            let mut directory_entry = GameDirectoryEntry{ game, name: game_name, gm: player_id, players: HashSet::new() };
            directory_entry.players.insert(player_id);
            self.games.insert(game_id, directory_entry);
            Ok(())
        }
        else
        {
            debug!("Player is not registered.");
            Err(())
        }
    }

    pub fn get_mut_game(&'a mut self, id: &GameId) -> Option<&'a mut Game>
    {
        if let Some(dir_entry) = self.games.get_mut(id)
        {
            return Some(&mut dir_entry.game);
        }
        else
        {
            return None;
        }
    }

    pub fn get_game(&'a self, id: &GameId) -> Option<&'a Game>
    {
        let entry = self.games.get(id)?;

        Some(&entry.game)
    }

    pub fn register_player(&mut self, player_name: String, player_id: PlayerId, player_comm_channel: Sender<Arc<WhatChanged>>) -> Result<(), ()>
    {
        match self.players.entry(player_id)
        {
            MapEntry::Occupied(_) => Err(()),
            MapEntry::Vacant(vacant) => 
            {
                vacant.insert(PlayerDirectoryEntry 
                {
                    player_name,
                    player_id, player_games: HashSet::new(), 
                    player_characters: HashMap::new(), 
                    player_sender: player_comm_channel 
                });
                Ok(())
            },
        }
    }

    pub fn join_game(&mut self, player_id: PlayerId, game_id: GameId) -> Result<(), ()>
    {
        debug!("Starting join_game() for player_id {} and game id {}", player_id, game_id);
        if self.games.contains_key(&game_id) && self.players.contains_key(&player_id)
        {
            debug!("Game id and player id match.");
            let game_dir = self.games.get_mut(&game_id).unwrap();
            let player_dir = self.players.get_mut(&player_id).unwrap();

            game_dir.players.insert(player_id);
            player_dir.player_games.insert(game_id);

            Ok(())
        }
        else
        {
            debug!("Game id matched: {}", self.games.contains_key(&game_id));
            debug!("Player id matched: {}", self.players.contains_key(&player_id));
            Err(())
        }
    }

    pub fn get_player_sender(&self, player_id: &PlayerId) -> Option<Sender<Arc<WhatChanged>>>
    {
        if let Some(players) = self.players.get(&player_id)
        {
            Some(players.player_sender.clone())
        }
        else
        {
            None
        }
    }

    pub fn gm_id(&self, game_id:  &GameId) -> Option<&PlayerId>
    {
        if let Some(game_entry) = self.games.get(game_id)
        {
            Some(&game_entry.gm)
        }
        else {
            None
        }
    }

    pub fn gm_sender(&self, game_id: &GameId) -> Option<Sender<Arc<WhatChanged>>>
    {
        if let Some(gm_id) = self.gm_id(game_id)
        {
            if let Some(player_entry) = self.players.get(gm_id)
            {
                Some(player_entry.player_sender.clone())
            }
            else {
                None
            }
        }
        else {
            None
        }
    }

    pub fn game_has_player(&self, game_id: &GameId, player_id: &PlayerId) -> bool
    {
        self.games.contains_key(game_id) && self.games.get(game_id).unwrap().players.contains(player_id)
    }

    pub fn player_in_game(&self, player_id: PlayerId, game_id: GameId) -> bool
    {
        self.players.contains_key(&player_id) && self.players.get(&player_id).unwrap().player_games.contains(&game_id)
    }

    pub fn games_by_player(&self, player_id: PlayerId) -> Option<&HashSet<GameId>>
    {
        let player_entry = self.players.get(&player_id)?;

        Some(&player_entry.player_games)
    }

    pub fn player_name(&self, player_id: &PlayerId) -> Option<&str>
    {
        let player_entry = self.players.get(player_id)?;

        Some(player_entry.player_name.as_str())
    }

    pub fn players_by_game(&self, game_id: &GameId) -> Option<&HashSet<PlayerId>>
    {
        if self.games.contains_key(game_id)
        {
            Some(&self.games.get(game_id).unwrap().players)
        }
        else
        {
            None
        }
    }

    pub fn leave_game(&mut self, player_id: PlayerId, game_id: GameId) -> Result<(), ()>
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

    pub fn enumerate_games(&self) -> HashSet<GameId>
    {
        let mut result = HashSet::new();

        self.games.keys().for_each(|f| {result.insert(*f);});

        return result;
    }

    pub fn enumerate_players(&self) -> HashSet<PlayerId>
    {
        let mut result = HashSet::new();

        self.players.keys().for_each(|f|{result.insert(*f);});

        return result;
    }

    pub fn delete_game(&mut self, game_id: GameId) -> Result<GameDirectoryEntry, ()>
    {
        if let Some(game) = self.games.remove(&game_id)
        {
            for player in game.players.iter()
            {
                match self.players.entry(*player)
                {
                    MapEntry::Occupied(mut player_entry) => {
                        player_entry.get_mut().player_games.remove(&game_id);
                    },
                    MapEntry::Vacant(_) => {}
                }
            }

            Ok(game)
        }
        else
        {
            return Err(())
        }
    }

    pub fn unregister_player(&mut self, player_id: PlayerId) -> Result<(), ()>
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

    pub fn is_registered(&self, player_id: &PlayerId) -> bool
    {
        self.players.contains_key(&player_id)
    }

    pub fn is_game(&self, game_id:  &GameId) -> bool
    {
        self.games.contains_key(game_id)
    }

    pub fn characters_by_player(&self, game_id: &GameId, player_id: &PlayerId) -> Option<&HashSet<CharacterId>>
    {
        match self.players.get(player_id)
        {
            Some(entry) => {
                match entry.player_characters.get(game_id)
                {
                    Some(characters) => Some(&characters),
                    None => None,
                }
            },
            None => None,
        }
    }

    pub fn add_character(&mut self, player_id: &PlayerId, game_id: &GameId, character: Character) -> Option<CharacterId>
    {
        match (self.players.get_mut(player_id), self.games.get_mut(game_id)) {
            (Some(player_entry), Some(game)) => {
                let character_id = game.game.add_cast_member(character);
                player_entry.player_characters.entry(*game_id).or_insert(HashSet::new()).insert(character_id.clone());
                Some(character_id)
            }, 
            _ => {
                None
            }, 
        }
    }

    pub fn players_by_character(&self, game_id: &GameId, char_id: &CharacterId) -> Option<&PlayerId>
    {
        self.players.iter().find(|p| 
            p.1.player_characters.contains_key(game_id) && p.1.player_characters.get(game_id).unwrap().contains(char_id)
        ).map(|p| p.0)   
    }

    pub fn is_gm(&self, player_id: &PlayerId, game_id: &GameId) -> bool
    {
        match self.games.get(game_id)
        {
            Some(game_entry) => {
                game_entry.gm == *player_id
            },
            None => {
                false
            }
        }
    }

    pub fn get_player_name(&self, player_id: &PlayerId) -> Option<String>
    {
        let dir_entry = self.players.get(player_id)?;
        Some(dir_entry.player_name.clone())
    }

    pub fn get_game_name(&self, game_id: &GameId) -> Option<String>
    {
        debug!("Request retrieved to retrieve game name for id {}", game_id);
        let dir_entry = self.games.get(game_id)?;
        debug!("Game name found: {}", dir_entry.name);
        Some(dir_entry.name.clone())
    }
}

#[cfg(test)]
pub mod tests
{
    use std::{collections::HashSet, sync::Arc};

    use tokio::sync::mpsc::{channel, Sender};
    use uuid::Uuid;

    use crate::{tracker::{game::Game, character::Character}, gamerunner::{WhatChanged, PlayerId, CharacterId}};

    use super::GameRegistry;

    pub fn init()
    {
        let _ = env_logger::builder().is_test(true).try_init();
    }


    #[test]
    pub fn if_a_registry_holds_a_valid_game_for_a_given_id_then_get_mut_game_return_a_mutable_ref_wrapped_in_ok()
    {
        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let game: Game = Game::new();
        let id = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), id, game).is_ok());

        assert!(registry.get_mut_game(&id).is_some());
    }

    #[test]
    pub fn if_a_registry_does_not_hold_a_valid_game_for_some_id_then_get_mut_game_will_return_none()
    {
        let mut registry = GameRegistry::new();

        let id = Uuid::new_v4();

        assert!(registry.get_mut_game(&id).is_none());
    }

    #[test]
    pub fn if_a_registry_holds_a_valid_game_for_some_given_id_then_get_game_returns_a_borrow_wrapped_in_ok()
    {
        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let game = Game::new();
        let id = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), id, game).is_ok());

        assert!(registry.get_game(&id).is_some() );
    }
    
    #[test]
    pub fn if_a_registry_does_not_hold_a_valid_game_for_some_given_id_then_get_game_returns_none()
    {
        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let game = Game::new();
        let id = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), id, game).is_ok());

        assert!(registry.get_game(&Uuid::new_v4()).is_none());
    }

    #[test]
    pub fn a_player_who_registers_will_receive_an_ok_if_registration_completed()
    {
        let mut registry = GameRegistry::new();

        let player_id = Uuid::new_v4();
        let player_name = String::from("Lizard");
        let (sender, _) = channel(32);

        assert!(registry.register_player(player_name, player_id, sender).is_ok())
    }

    #[test]
    pub fn a_duplicate_player_id_may_not_be_used_to_register_a_new_player()
    {
        let mut registry = GameRegistry::new();

        let player_id = Uuid::new_v4();
        let player_name = String::from("Lizard");
        let (sender, _) = channel(32);
        
        assert!(registry.register_player(player_name.clone(), player_id, sender.clone()).is_ok());
        assert!(registry.register_player(player_name, player_id, sender).is_err());
    }

    #[test]
    pub fn a_duplicate_name_may_be_used_on_multiple_players()
    {
        let mut registry = GameRegistry::new();

        let player_1_id = PlayerId::new_v4();
        let player_1_name = String::from("Seamus");
        let (player_1_sender, _) = channel(32);

        let player_2_id = PlayerId::new_v4();
        let player_2_name = String::from("Seamus");
        let (player_2_sender, _) = channel(32);

        assert!(registry.register_player(player_1_name, player_1_id, player_1_sender).is_ok());
        assert!(registry.register_player(player_2_name, player_2_id, player_2_sender).is_ok());
    }

    #[test]
    pub fn a_registered_player_becomes_the_gm_of_any_game_they_create()
    {
        let mut registry = GameRegistry::new();

        let player_id = PlayerId::new_v4();
        let player_name = String::from("King Ghidorah");
        let (player_notification_channel, _) = channel(32);

        assert!(registry.register_player(player_name, player_id, player_notification_channel).is_ok());

        let game_1_id = Uuid::new_v4();
        let game_2_id = Uuid::new_v4();

        assert!(registry.new_game(player_id, String::from("Megasaurus Wrex"), game_1_id, Game::new()).is_ok());
        assert!(registry.new_game(player_id, String::from("Duplicate THIS"), game_2_id, Game::new()).is_ok());

        assert!(registry.is_gm(&player_id, &game_1_id));
        assert!(registry.is_gm(&player_id, &game_2_id));
    }

    #[test]
    pub fn players_who_is_registered_may_join_a_game()
    {
        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let player_id = Uuid::new_v4();
        let player_name = String::from("Lizard");
        let (sender, _) = channel(32);
        let game_id = Uuid::new_v4();
        let game = Game::new();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_id, game).is_ok());
        assert!(registry.register_player(player_name, player_id, sender).is_ok());

        assert!(registry.join_game(player_id, game_id).is_ok());
    }

    #[test]
    pub fn a_player_who_is_not_registered_will_generate_err_when_joining_a_game()
    {
        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let player_id = Uuid::new_v4();
        let game_id = Uuid::new_v4();
        let game = Game::new();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_id, game).is_ok());

        assert!(registry.join_game(player_id, game_id).is_err());
    }

    #[tokio::test]
    pub async fn the_id_of_a_games_gm_may_be_retrieved_with_gm_id()
    {
        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let game_id = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_id, Game::new()).is_ok());

        assert!(registry.gm_id(&game_id).is_some());
        let retrieved_id: &PlayerId = registry.gm_id(&game_id).unwrap();

        assert_eq!(gm, *retrieved_id);
    }

    #[tokio::test]
    pub async fn a_gms_sending_channel_may_be_retrieved_with_gm_sender()
    {
        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, mut gm_receiver) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let game_id = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_id, Game::new()).is_ok());

        assert!(registry.gm_sender(&game_id).is_some());
        let sender: Sender<Arc<WhatChanged>> = registry.gm_sender(&game_id).unwrap();

        assert!(sender.send(Arc::from(WhatChanged::StartingCombatRound)).await.is_ok());

        let sent_message = gm_receiver.recv().await;
        assert!(sent_message.is_some());
        match sent_message.unwrap().as_ref()
        {
            WhatChanged::StartingCombatRound => {}
            _ => {panic!("The wrong WhatChanged was sent.")}
        }

    }

    #[tokio::test]
    pub async fn a_players_sending_channel_may_be_retrieved_with_the_player_id()
    {
        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let player_id = Uuid::new_v4();
        let player_name = String::from("Lizard");
        let (sender, mut receiver) = channel(32);
        let game_id = Uuid::new_v4();
        let game = Game::new();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_id, game).is_ok());
        assert!(registry.register_player(player_name, player_id, sender).is_ok());

        let player_comms = registry.get_player_sender(&player_id).unwrap();
        
        
        assert!(player_comms.send(Arc::new(crate::gamerunner::WhatChanged::CombatEnded)).await.is_ok());

        assert!(receiver.recv().await.is_some());
    }

    #[test]
    pub fn a_player_may_add_a_character_to_a_game()
    {
        init();
        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let game_1 = Uuid::new_v4();

        let player_1 = Uuid::new_v4();
        let player_name = String::from("Lizard");
        let (player_sender, _) = channel(32);
        let mork = Character::new_pc(crate::tracker::character::Metatypes::Orc, String::from("Orcifer"));

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());
        
        assert!(registry.register_player(player_name, player_1, player_sender).is_ok());
        assert!(registry.join_game(player_1, game_1).is_ok());
    
        let char_id: Option<CharacterId> = registry.add_character(&player_1, &game_1, mork);

        assert!(char_id.is_some());
        assert_eq!(1, registry.get_game(&game_1).unwrap().cast_size());
        assert!(registry.get_game(&game_1).unwrap().get_cast_by_id(&char_id.unwrap()).is_some());
    }

    #[test]
    pub fn a_player_may_retrieve_their_character_ids_from_a_given_game()
    {
        init();
        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let game_1 = Uuid::new_v4();
        let game_2 = Uuid::new_v4();

        let player_1 = Uuid::new_v4();
        let player_name = String::from("Lizard");
        let (player_sender, _) = channel(32);
        let dorf = Character::new_pc(crate::tracker::character::Metatypes::Dwarf, String::from("Dorf"));
        let mork = Character::new_pc(crate::tracker::character::Metatypes::Orc, String::from("Mork"));

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.register_player(player_name, player_1, player_sender).is_ok());

        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_2, Game::new()).is_ok());

        assert!(registry.join_game(player_1, game_1).is_ok());
        assert!(registry.join_game(player_1, game_2).is_ok());

        let dorf_id = registry.add_character(&player_1, &game_1, dorf);
        let mork_id = registry.add_character(&player_1, &game_2, mork);

        let mut chars = registry.characters_by_player(&game_1, &player_1);
        assert!(chars.is_some());
        assert!(chars.unwrap().contains(&dorf_id.unwrap()));
        assert!(!chars.unwrap().contains(&mork_id.unwrap()));

        chars = registry.characters_by_player(&game_2, &player_1);
        assert!(chars.is_some());
        assert!(!chars.unwrap().contains(&dorf_id.unwrap()));
        assert!(chars.unwrap().contains(&mork_id.unwrap()));
    }

    #[test]
    pub fn a_player_may_enumerate_the_games_they_are_in()
    {
        init();
        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let game_1 = Uuid::new_v4();
        let game_2 = Uuid::new_v4();
        let game_3 = Uuid::new_v4();

        let (sender, _) = channel(32);
        let player_1 = Uuid::new_v4();
        let player_name = String::from("Lizard");

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_2, Game::new()).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_3, Game::new()).is_ok());
        assert!(registry.register_player(player_name, player_1, sender).is_ok());

        assert!(registry.join_game(player_1, game_1).is_ok());
        assert!(registry.join_game(player_1, game_2).is_ok());
        assert!(registry.join_game(player_1, game_3).is_ok());

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
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let game_1 = Uuid::new_v4();
        let game_2 = Uuid::new_v4();
        let game_3 = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_2, Game::new()).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_3, Game::new()).is_ok());

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

        let registry = GameRegistry::new();

        let games = registry.enumerate_games();

        assert_eq!(0, games.len());
    }

    #[test]
    pub fn a_full_list_of_registered_players_may_be_retrieved_with_enumerate_player()
    {
        init();
        let mut registry = GameRegistry::new();
        let player_1 = Uuid::new_v4();
        let player_1_name = String::from("Lizard");
        let (sender_1, _) = channel(32);
        let player_2 = Uuid::new_v4();
        let player_2_name = String::from("Wizard");
        let (sender_2, _) = channel(32);

        assert!(registry.register_player(player_1_name, player_1, sender_1).is_ok());
        assert!(registry.register_player(player_2_name, player_2, sender_2).is_ok());

        let registered_players = registry.enumerate_players();

        assert!(registered_players.len() == 2);
        assert!(registered_players.contains(&player_1));
        assert!(registered_players.contains(&player_2));
    }

    #[test]
    pub fn if_no_players_have_registered_enumerate_players_returns_an_empty_set()
    {
        init();
        let registry = GameRegistry::new();

        let registered_players = registry.enumerate_players();

        assert_eq!(0, registered_players.len());
    }

    #[test]
    pub fn if_the_argument_to_games_by_player_does_not_resolve_to_a_registered_player_none_is_returned()
    {
        init();
        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let game_1 = Uuid::new_v4();

        let (sender, _) = channel(32);
        let player_1 = Uuid::new_v4();
        let player_name = String::from("Lizard");

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());
        assert!(registry.register_player(player_name, player_1, sender).is_ok());

        assert!(registry.games_by_player(Uuid::new_v4()).is_none());

    }

    #[test]
    pub fn if_a_player_is_not_in_any_games_then_games_by_player_will_return_an_empty_set()
    {
        init();
        let mut registry = GameRegistry::new();
        let game_1 = Uuid::new_v4();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());

        let player_1 = Uuid::new_v4();
        let player_name = String::from("Lizard");
        let (sender, _) = channel(32);
        assert!(registry.register_player(player_name, player_1, sender).is_ok());

        let game = registry.games_by_player(player_1).unwrap();

        assert_eq!(0, game.len());
    }

    #[test]
    pub fn a_game_may_enumerate_the_players_who_have_joined_it()
    {
        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let game_1 = Uuid::new_v4();

        let player_1 = Uuid::new_v4();
        let player_1_name = String::from("Lizard");
        let (sender_1, _) = channel(32);
        let player_2 = Uuid::new_v4();
        let player_2_name = String::from("Wizard");
        let (sender_2, _) = channel(32);
        let player_3 = Uuid::new_v4();
        let player_3_name = String::from("Gizzard");
        let (sender_3, _) = channel(32);

        assert!(registry.register_player(player_1_name, player_1, sender_1).is_ok());
        assert!(registry.register_player(player_2_name, player_2, sender_2).is_ok());
        assert!(registry.register_player(player_3_name, player_3, sender_3).is_ok());

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());

        assert!(registry.join_game(player_1, game_1).is_ok());
        assert!(registry.join_game(player_2, game_1).is_ok());
        assert!(registry.join_game(player_3, game_1).is_ok());

        let players: &HashSet<Uuid> = registry.players_by_game(&game_1).unwrap();

        assert_eq!(4, players.len());
        assert!(players.contains(&gm));
        assert!(players.contains(&player_1));
        assert!(players.contains(&player_2));
        assert!(players.contains(&player_3));
    }

    #[test]
    pub fn if_a_game_exists_but_has_no_players_then_players_by_game_will_contain_only_the_gm_as_an_entry()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let game_1 = Uuid::new_v4();
        let player_1 = Uuid::new_v4();
        let player_name = String::from("Lizard");
        let (sender_1, _) = channel(32);

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());
        assert!(registry.register_player(player_name, player_1, sender_1).is_ok());
        
        let players = registry.players_by_game(&game_1);
        assert!(players.is_some());
        assert!(players.unwrap().len() == 1);
        assert!(players.unwrap().contains(&gm));
    }


    #[test]
    pub fn if_the_argument_to_players_by_game_does_not_resolve_to_a_real_game_then_none_is_returned()
    {
        init();
        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");
        
        let game_1 = Uuid::new_v4();

        let player_1 = Uuid::new_v4();
        let player_name = String::from("Lizard");
        let (sender, _) = channel(32);
    
        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());
        assert!(registry.register_player(player_name, player_1, sender).is_ok());

        assert!(registry.join_game(player_1, game_1).is_ok());

        assert!(registry.players_by_game(&Uuid::new_v4()).is_none());
    }

    #[test]
    pub fn if_a_player_leaves_a_game_both_the_player_and_the_game_records_update_to_remove_the_player()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let player_1 = Uuid::new_v4();
        let player_1_name = String::from("Lizard");
        let player_2 = Uuid::new_v4();
        let player_2_name = String::from("Wizard");
        let (mut sender, _) = channel(32);
        let game_id = Uuid::new_v4();
        let game = Game::new();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_id, game).is_ok());

        assert!(registry.register_player(player_1_name, player_1, sender).is_ok());
        (sender, _) = channel(32);
        assert!(registry.register_player(player_2_name, player_2, sender).is_ok());

        assert!(registry.join_game(player_1, game_id).is_ok());
        assert!(registry.join_game(player_2, game_id).is_ok());

        assert!(registry.leave_game(player_1, game_id).is_ok());
        assert!(!registry.game_has_player(&game_id, &player_1));
        assert!(!registry.player_in_game(player_1, game_id));
    }

    #[test]
    pub fn if_a_player_leaves_a_game_they_are_not_a_member_of_leave_game_returns_err()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let player_1 = Uuid::new_v4();
        let game_id = Uuid::new_v4();
        let game = Game::new();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_id, game).is_ok());

        assert!(registry.leave_game(player_1, game_id).is_err());
    }

    #[test]
    pub fn when_a_game_is_deleted_all_players_are_updated_to_remove_that_game_from_their_lists()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let player_1 = Uuid::new_v4();
        let player_1_name = String::from("Lizard");
        let (sender_1, _) = channel(32);
        let player_2 = Uuid::new_v4();
        let player_2_name = String::from("Wizard");
        let (sender_2, _) = channel(32);
        let player_3 = Uuid::new_v4();
        let player_3_name = String::from("Gizard");
        let (sender_3, _) = channel(32);
        
        let game_id = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_id, Game::new()).is_ok());
        assert!(registry.register_player(player_1_name, player_1, sender_1).is_ok());
        assert!(registry.register_player(player_2_name, player_2, sender_2).is_ok());
        assert!(registry.register_player(player_3_name, player_3, sender_3).is_ok());

        assert!(registry.join_game(player_1, game_id).is_ok());
        assert!(registry.join_game(player_2, game_id).is_ok());
        assert!(registry.join_game(player_3, game_id).is_ok());

        assert!(registry.delete_game(game_id).is_ok());

        assert!(registry.players_by_game(&game_id).is_none());
        assert!(!registry.player_in_game(player_1, game_id));
        assert!(!registry.player_in_game(player_2, game_id));
        assert!(!registry.player_in_game(player_3, game_id));
        
    }

    
    #[test]
    pub fn when_delete_game_is_passed_an_id_that_does_not_represent_a_real_game_err_is_returned()
    {
        init();

        let mut registry = GameRegistry::new();

        assert!(registry.delete_game(Uuid::new_v4()).is_err());
    }

    #[test]
    pub fn when_a_player_unregisters_they_are_removed_from_all_games_they_are_part_of()
    {
        init();
        let  mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let player_1 = Uuid::new_v4();
        let player_1_name = String::from("Lizard");
        let (sender_1, _) = channel(32);
        let player_2 = Uuid::new_v4();
        let player_2_name = String::from("Wizard");
        let (sender_2, _) = channel(32);

        let game_1 = Uuid::new_v4();
        let game_2 = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_2, Game::new()).is_ok());
        assert!(registry.register_player(player_1_name, player_1, sender_1).is_ok());
        assert!(registry.register_player(player_2_name, player_2, sender_2).is_ok());

        assert!(registry.join_game(player_1, game_1).is_ok());
        assert!(registry.join_game(player_2, game_1).is_ok());
        assert!(registry.join_game(player_1, game_2).is_ok());

        assert!(registry.unregister_player(player_1).is_ok());

        let players = registry.players_by_game(&game_1);
        assert!(players.is_some() && players.unwrap().contains(&player_2));
        assert!(!players.unwrap().contains(&player_1));

        let players = registry.players_by_game(&game_2);
        assert!(players.is_some());
        assert!(!players.unwrap().contains(&player_1));
    }

    #[test]
    pub fn when_an_unrecognized_player_id_is_handed_to_unregister_err_is_returned()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let player_1 = Uuid::new_v4();
        let player_1_name = String::from("Lizard");
        let (sender_1, _) = channel(32);
        let player_2 = Uuid::new_v4();
        let player_2_name = String::from("Wizard");
        let (sender_2, _) = channel(32);

        let game_1 = Uuid::new_v4();
        let game_2 = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_2, Game::new()).is_ok());
        assert!(registry.register_player(player_1_name, player_1, sender_1).is_ok());
        assert!(registry.register_player(player_2_name, player_2, sender_2).is_ok());

        let result = registry.unregister_player(Uuid::new_v4());

        assert!(result.is_err());
    }

    #[test]
    pub fn is_registered_will_return_true_if_a_given_id_is_registered_with_the_directory()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let player_1 = Uuid::new_v4();
        let player_name = String::from("Lizard");
        let (sender_1, _) = channel(32);

        let game_1 = Uuid::new_v4();

        assert!((registry.register_player(player_name, player_1, sender_1).is_ok()));
        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());

        assert!(registry.join_game(player_1, game_1).is_ok());

        assert!(registry.is_registered(&player_1));
    }

    #[test]
    pub fn is_registered_will_return_false_if_a_given_id_is_not_registered_with_the_directory()
    {
        init();

        let registry = GameRegistry::new();

        assert!(!registry.is_registered(&Uuid::new_v4()));
    }

    #[test]
    pub fn is_registered_returns_true_regardless_of_whether_a_player_has_joined_any_game()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let player_1 = Uuid::new_v4();
        let player_1_name = String::from("Lizard");
        let (sender_1, _) = channel(32);
        let player_2 = Uuid::new_v4();
        let player_2_name = String::from("Wizard");
        let (sender_2, _) = channel(32);

        let game_1 = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());
        assert!(registry.register_player(player_1_name, player_1, sender_1).is_ok());
        assert!(registry.register_player(player_2_name, player_2, sender_2).is_ok());

        assert!(registry.join_game(player_1, game_1).is_ok());

        assert!(registry.is_registered(&player_1));
        assert!(registry.is_registered(&player_2));
    }

    #[test]
    pub fn game_has_player_will_return_false_if_the_game_id_is_unknown()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let player_1 = Uuid::new_v4();
        let player_1_name = String::from("Lizard");
        let (sender_1, _) = channel(32);

        let game_1 = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());
        assert!(registry.register_player(player_1_name, player_1, sender_1).is_ok());

        assert!(!registry.game_has_player(&Uuid::new_v4(), &player_1))
    }

    #[test]
    pub fn game_has_player_will_return_false_if_the_player_id_is_unknown()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let player_1 = Uuid::new_v4();
        let player_name = String::from("Lizard");
        let (sender_1, _) = channel(32);

        let game_1 = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());
        assert!(registry.register_player(player_name, player_1, sender_1).is_ok());

        assert!(!registry.game_has_player(&game_1, &Uuid::new_v4()));
    }

    #[test]
    pub fn game_has_player_will_return_false_if_the_player_id_is_registered_and_has_joined_a_game_but_not_the_one_being_asked_about()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let player_1 = Uuid::new_v4();
        let player_1_name = String::from("Lizard");
        let player_2 = Uuid::new_v4();
        let player_2_name = String::from("Wizard");
        let (sender_1, _) = channel(32);
        let (sender_2, _) = channel(32);

        let game_1 = Uuid::new_v4();
        let game_2 = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_2, Game::new()).is_ok());

        assert!(registry.register_player(player_1_name, player_1, sender_1).is_ok());
        assert!(registry.register_player(player_2_name, player_2, sender_2).is_ok());

        assert!(registry.join_game(player_1, game_1).is_ok());
        assert!(registry.join_game(player_2, game_2).is_ok());

        assert!(!registry.game_has_player(&game_2, &player_1));
    }

    #[test]
    pub fn game_has_player_returns_true_when_a_player_id_is_a_player_in_the_game_being_queried()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let player_1 = Uuid::new_v4();
        let player_name = String::from("Lizard");
        let (sender_1, _) = channel(32);

        let game_1 = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok()); 

        assert!(registry.register_player(player_name, player_1, sender_1).is_ok());

        assert!(registry.join_game(player_1, game_1).is_ok());

        assert!(registry.game_has_player(&game_1, &player_1));
    }

    #[test]
    pub fn player_in_game_will_return_false_if_either_the_game_or_the_player_id_is_unknown()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let player_1 = Uuid::new_v4();
        let player_name = String::from("Lizard");
        let (sender_1, _) = channel(32);

        let game_1 = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());
        assert!(registry.register_player(player_name, player_1, sender_1).is_ok());

        assert!(!registry.player_in_game(Uuid::new_v4(), Uuid::new_v4()));
    }

    #[test]
    pub fn is_game_will_return_true_if_the_game_id_maps_to_a_real_game_directory_entry()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");
        
        let game_1 = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());
        
        assert!(registry.is_game(&game_1));
    }

    #[test]
    pub fn is_game_will_return_false_if_the_game_id_does_not_map_to_a_real_game_directory_entry()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let game_1 = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());

        assert!(!registry.is_game(&Uuid::new_v4()));
    }

    #[test]
    pub fn is_gm_will_retur_false_if_the_game_id_does_not_map_to_a_real_game_directory_entry()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let game_1 = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());
    }

    #[test]
    pub fn is_gm_will_return_false_if_the_game_id_maps_but_the_provided_player_id_is_not_the_gm()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let game_1 = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());

        let player_1 = PlayerId::new_v4();
        let player_name = String::from("Lizard");
        let (player_1_sender, _) = channel(32);

        assert!(registry.register_player(player_name, player_1, player_1_sender).is_ok());
        assert!(registry.join_game(player_1, game_1).is_ok());

        assert!(!registry.is_gm(&player_1, &game_1));
    }

    #[test]
    pub fn is_gm_will_return_true_if_the_game_id_maps_to_a_valid_game_entry_and_the_player_id_matches_the_gm_field()
    {
        init();

        let mut registry = GameRegistry::new();
        let gm = PlayerId::new_v4();
        let (gm_sender, _) = channel(32);
        let gm_name = String::from("King Ghidorah");

        let game_1 = Uuid::new_v4();

        assert!(registry.register_player(gm_name, gm, gm_sender).is_ok());
        assert!(registry.new_game(gm, String::from("Made up"), game_1, Game::new()).is_ok());

        let player_1 = PlayerId::new_v4();
        let player_name = String::from("Lizard");
        let (player_1_sender, _) = channel(32);

        assert!(registry.register_player(player_name, player_1, player_1_sender).is_ok());
        assert!(registry.join_game(player_1, game_1).is_ok());

        assert!(registry.is_gm(&gm, &game_1));
        
    }

    #[test]
    pub fn get_player_name_will_return_some_an_owned_string_value_holding_the_player_name_if_the_id_is_valid()
    {
        init();

        let mut registry = GameRegistry::new();
        let player_id = PlayerId::new_v4();
        let (player_sender, _) = channel(32);
        let player_name = String::from("Guess Who");

        assert!(registry.register_player(player_name.clone(), player_id, player_sender).is_ok());
        
        let retrieved_name: Option<String> = registry.get_player_name(&player_id);

        assert!(retrieved_name.is_some());
        assert_eq!(retrieved_name.unwrap(), player_name);
    }

    #[test]
    pub fn get_player_name_will_return_none_if_the_player_id_is_not_registered()
    {
        init();

        let mut registry = GameRegistry::new();
        let player_id = PlayerId::new_v4();
        let (player_sender, _) = channel(32);
        let player_name = String::from("Charles Darwin");

        assert!(registry.register_player(player_name, player_id, player_sender).is_ok());

        let retrieved_name: Option<String> = registry.get_player_name(&Uuid::new_v4());

        assert!(retrieved_name.is_none());
    }

    #[test]
    pub fn get_game_will_return_some_wrapping_the_name_if_the_game_id_maps_to_a_real_game()
    {
        init();

        let mut registry = GameRegistry::new();
        let player_id = PlayerId::new_v4();
        let (player_sender, _) = channel(32);
        let player_name = String::from("The Wizard");

        assert!(registry.register_player(player_name, player_id.clone(), player_sender).is_ok());

        let game_id = Uuid::new_v4();
        let game_name = String::from("Rocks Fall");
        
        assert!(registry.new_game(player_id, game_name, game_id.clone(), Game::new()).is_ok());

        assert!(registry.get_game_name(&game_id).is_some());
    }

    #[test]
    pub fn get_game_name_will_return_none_if_the_game_id_does_not_map_to_a_game()
    {
        init();

        let mut registry = GameRegistry::new();
        let player_id = PlayerId::new_v4();
        let (player_sender, _) = channel(32);
        let player_name = String::from("The Wizard");

        assert!(registry.register_player(player_name, player_id, player_sender).is_ok());

        let game_id = Uuid::new_v4();
        let game_name = String::from("Rocks Fall");
        
        assert!(registry.new_game(player_id, game_name, game_id, Game::new()).is_ok());

        assert!(registry.get_game_name(&Uuid::new_v4()).is_none());
    }
}