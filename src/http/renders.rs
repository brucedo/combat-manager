use std::str::FromStr;

use log::debug;
use rocket::{get, post, State, response::Redirect, uri, form::{FromForm, Form}, http::hyper::Uri};
use rocket_dyn_templates::{Template, context};
use uuid::Uuid;
use tokio::sync::{mpsc::Sender, oneshot::channel};

use crate::{gamerunner::{Message, Event, Outcome}, http::session::NewSessionOutcome};

use super::{models::{GameSummary, GameSummaries, GMView, IndexModel}, errors::Error, session::Session, metagame::Metagame};

#[get("/")]
pub async fn index(state: &State<Metagame>, session: Session) -> Result<Template, Error>
{

    let my_sender = state.game_runner_pipe.clone();
    let (their_sender, my_receiver) = channel();
    let msg = Message {game_id: Uuid::new_v4(), msg: Event::Enumerate, reply_channel: their_sender};

    if let Err(err) = my_sender.send(msg).await
    {
        return Err(Error::InternalServerError(Template::render("error_pages/500", context! {action_name: "create a new game", error: err.to_string()})));
    }

    let mut summaries = Vec::<GameSummary>::new();
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
                        summaries.push(GameSummary { game_name: name, game_id: id })
                    }
                },
                _ => { }
            }
        },
        Err(_) => {todo!()},
    }

    let model = IndexModel { player_handle: &session.handle_as_ref(), summaries  };


    return Ok(Template::render("index", model));
}

#[post("/game")]
pub async fn create_game(state: &State<Metagame>, session: Session) -> Result<Redirect, Error>
{
    let my_sender = state.game_runner_pipe.clone();

    let (their_sender, my_receiver) = channel();
    let msg = Message { game_id: Uuid::new_v4(), reply_channel: their_sender, msg: Event::New };

    if let Err(err) = my_sender.send(msg).await
    {
        return Err(Error::InternalServerError(Template::render("error_pages/500", context! {action_name: "create a new game", error: err.to_string()})));
    }

    match my_receiver.await
    {
        Ok(response) => 
        {
            match response
            {
                Outcome::Created(game_id) =>
                {   
                    state.new_game(game_id, session.player_id(), String::from("game_name"), Uri::from(format!("/game/{}", game_id)).unwrap_or_else(|| panic!("BOOOOM")));
                    let lock = state.game_details.write();
                    Ok(Redirect::to(uri!(game_view(game_id))))
                }
                _ =>
                {
                    let err = "Boy howdy, something really went south here.  We received a completely unexpected message type from the GameRunner for creating a game.";
                    return Err(Error::InternalServerError(Template::render("error_pages/500", context! {action_name: "create a new game", error: err})));
                }
            }
        },
        Err(err) => 
        {
            return Err(Error::InternalServerError(Template::render("error_pages/500", context! {action_name: "create a new game", error: err.to_string()})));
        },
    }

}

#[get("/game/<id>")]
pub async fn game_view(id: Uuid, state: &State<Metagame>) -> Template
{

    return Template::render("gm_view", GMView{game_id: id});
}

#[get("/<_..>")]
pub async fn no_session() -> Template
{
    Template::render("register", context!{})
}

#[derive(FromForm)]
pub struct UserHandle<'r> {
    #[field(name = "player_handle")]
    player_handle: &'r str
}

#[post("/gen_session", data = "<submission>")]
pub async fn new_session(_proof_of_session: NewSessionOutcome, session: Session, submission: Form<UserHandle<'_>>) -> Redirect
// #[post("/gen_session")]
// pub async fn new_session(proof_of_session: NewSessionOutcome, session: Session) -> Redirect
{
    session.set_handle(String::from(submission.player_handle));
    Redirect::to(uri!("/"))
}