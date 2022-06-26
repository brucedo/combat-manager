use rocket::serde::{Serialize, self, Deserialize};
use uuid::Uuid;



#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct NewGameJson
{
    pub game_id: Uuid,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Character
{

}
