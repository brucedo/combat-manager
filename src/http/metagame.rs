use std::collections::HashMap;

use log::debug;
use parking_lot::RwLock;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::gamerunner::dispatcher::Message;


pub struct Metagame<'s>
{
    pub game_runner_pipe: Sender<Message>,
    pub game_details: RwLock<HashMap<Uuid, GameAdditionalInformation<'s>>>,
}

impl<'s> Metagame<'s>
{
    pub fn new<'a>(my_channel: Sender<Message>) -> Metagame<'a>
    {
        Metagame { game_runner_pipe: my_channel, game_details: RwLock::new(HashMap::new())}
    }

    pub fn new_game(&self, game_id: Uuid, gm_id: Uuid, game_name: String)
    {
        let mut detail_set = self.game_details.write();

        detail_set.insert(game_id, GameAdditionalInformation{gm_id, game_name, gamer_ref: "Tough"});
    }

    pub fn validate_ownership(&self, player_id: Uuid, game_id: Uuid) -> bool
    {
        debug!("Attempting to validate ownership of the game given by ID.");
        let lock = self.game_details.read();

        if let Some(game) = lock.get(&game_id)
        {
            debug!("Thisg game's GM id is: {}", game.gm_id);
            debug!("This player's id is: {}", player_id);
            return  game.gm_id == player_id;
        }
        else {return false};
    }

    pub fn game_name(&self, game_id: Uuid) -> Option<String>
    {
        let lock = self.game_details.read();

        let game = lock.get(&game_id)?;

        return Some(game.game_name.clone());
    }
}

pub struct GameAdditionalInformation<'a>
{
    pub gm_id: Uuid,
    pub game_name: String,
    pub gamer_ref: &'a str
}