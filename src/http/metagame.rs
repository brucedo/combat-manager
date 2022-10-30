use std::collections::HashMap;

use parking_lot::RwLock;
use rocket::http::hyper::Uri;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::gamerunner::Message;


pub struct Metagame
{
    pub game_runner_pipe: Sender<Message>,
    pub game_details: RwLock<HashMap<Uuid, GameAdditionalInformation>>
}

impl Metagame
{
    pub fn new(my_channel: Sender<Message>) -> Metagame
    {
        Metagame { game_runner_pipe: my_channel, game_details: RwLock::new(HashMap::new()) }
    }

    pub fn new_game(&self, game_id: Uuid, gm_id: Uuid, game_name: String, game_url: Uri)
    {
        let mut detail_set = self.game_details.write();

        detail_set.insert(game_id, GameAdditionalInformation{gm_id, game_name, game_url});
    }
}

pub struct GameAdditionalInformation
{
    pub gm_id: Uuid,
    pub game_name: String,
    pub game_url: Uri,

}