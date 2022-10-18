use log::debug;
use rocket::{get, State};
use rocket_dyn_templates::Template;
use tokio::sync::{mpsc::Sender, oneshot::channel};

use crate::gamerunner::Message;

#[get("/")]
pub fn index(state: &State<Sender<Message>>) //-> Template
{

}