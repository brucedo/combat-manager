mod api;

use log::debug;
use rocket::{State, http::Status, serde::{Serialize, self, json::Json}};
use tokio::sync::{mpsc::Sender};
use uuid::Uuid;
use rocket::get;

use crate::{gamerunner::{RequestMessage, ResponseMessage, NewGame}, dispatcher::api::NewGameJson};


#[get("/api/game/new")]
pub async fn new_game(state: &State<Sender<RequestMessage>>) -> Result<Json<NewGameJson>, (Status, &'static str)>
{
    debug!("Request received to generate new game.");
    let local_sender = state.clone();

    let (runner_sender, runner_receiver) = tokio::sync::oneshot::channel::<ResponseMessage>();
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
                            return Ok(Json(NewGameJson{game_id: id}));
                        },
                        ResponseMessage::Error(err) => {
                            debug!("Game creation error.  Message: {}", err.message);
                            return Err((Status::InternalServerError, "Game creation encountered some error."));
                        },
                        _ => {unreachable!()}
}
                },
                // The game runner actually does not return an error state here.
                Err(_) => todo!(),
            }
        },
        Err(_) => 
        {
            debug!("Blocking send failed on game create.  Channel may be defunct.");
            return Err((Status::InternalServerError, "Blocking send failed on game create request.  Channel may have closed."));
        },
    }
}
