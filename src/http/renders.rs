use log::debug;
use rocket::{get, post, State, response::Redirect, uri};
use rocket_dyn_templates::{Template, context};
use uuid::Uuid;
use tokio::sync::{mpsc::Sender, oneshot::channel};

use crate::gamerunner::{Message, Event, Outcome};

use super::serde::{GameSummary, GameSummaries, GMView};

#[get("/")]
pub async fn index(state: &State<Sender<Message>>) -> Template
{
    let my_sender = state.inner().clone();
    let (their_sender, my_receiver) = channel();
    let msg = Message {game_id: Uuid::new_v4(), msg: Event::Enumerate, reply_channel: their_sender};

    if let Err(_err) = my_sender.send(msg).await
    {

    }

    let mut model = Vec::<GameSummary>::new();
    match my_receiver.await
    {
        Ok(enum_outcome) => 
        {
            match enum_outcome
            {
                crate::gamerunner::Outcome::Summaries(summary) => 
                {
                    for (id, name) in summary
                    {
                        model.push(GameSummary { game_name: name, game_id: id })
                    }
                },
                _ => { }
            }
        },
        Err(_) => {todo!()},
    }


    return Template::render("index", GameSummaries{games: model});
}

#[post("/game")]
pub async fn create_game(state: &State<Sender<Message>>) -> Result<Redirect, Template>
{
    let my_sender = state.inner().clone();

    let (their_sender, my_receiver) = channel();
    let msg = Message { game_id: Uuid::new_v4(), reply_channel: their_sender, msg: Event::New };

    if let Err(err) = my_sender.send(msg).await
    {
        return Err(Template::render("error_pages/500", context! {action_name: "create a new game", error: err.to_string()}));
    }


    match my_receiver.await
    {
        Ok(response) => 
        {
            if let Outcome::Created(game_id) = response
            {   
                Ok(Redirect::to(uri!(gm_view(game_id))))
            }
            else
            {
                return Err(Template::render("error_pages/500", context! {action_name: "create a new game", error: "Unexpected response type from GameRunner."}));
            }
        },
        Err(err) => 
        {
            return Err(Template::render("error_pages/500", context! {action_name: "create a new game", error: err.to_string()}));
        },
    }

}

#[get("/admin/<id>")]
pub async fn gm_view(id: Uuid, state: &State<Sender<Message>>) -> Template
{

    return Template::render("gm_view", GMView{game_id: id});
}