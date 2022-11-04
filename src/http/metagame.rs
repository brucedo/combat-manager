use std::collections::HashMap;

use parking_lot::RwLock;
use rocket::http::uri::Origin;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::gamerunner::Message;


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

    pub fn new_game(&self, game_id: Uuid, gm_id: Uuid, game_name: String, game_url: Origin<'s>)
    {
        let mut detail_set = self.game_details.write();

        detail_set.insert(game_id, GameAdditionalInformation{gm_id, game_name, game_url});
    }

    pub fn validate_ownership(&self, player_id: Uuid, game_id: Uuid) -> bool
    {
        let lock = self.game_details.read();

        if let Some(game) = lock.get(&game_id)
        {
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
    pub game_url: Origin<'a>,

}