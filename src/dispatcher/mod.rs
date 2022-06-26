use log::debug;
use rocket::State;
use tokio::sync::{mpsc::{Sender, channel}};
use uuid::Uuid;
use rocket::get;

use crate::{MainMessages, gamerunner::{RequestMessage, ResponseMessage, NewGame}};


#[get("/api/game/new")]
pub async fn new_game(state: &State<Sender<RequestMessage>>)
{
    debug!("Request received to generate new game.");
    let local_sender = state.clone();

    let (runner_sender, mut runner_receiver) = tokio::sync::oneshot::channel::<ResponseMessage>();
    let msg = RequestMessage::New(NewGame{response: runner_sender});

    match local_sender.send(msg).await
    {
        Ok(_) => 
        {
            match runner_receiver.await
            {
                Ok(game_msg) => {
                    match game_msg
                    {
                        ResponseMessage::Created(id) => {
                            debug!("Game created.  ID: {}", id);
                        },
                        ResponseMessage::Error(err) => {
                            debug!("Game creation error.  Message: {}", err.message);
                        },
                        _ => {unreachable!()}
}
                },
                Err(_) => todo!(),
            }
        },
        Err(_) => 
        {
            debug!("Blocking send failed on game create.  Channel may be defunct.");
        },
    }
}
