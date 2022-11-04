use std::str::FromStr;

use log::debug;
use rocket::{get, post, State, response::Redirect, uri, form::{FromForm, Form}};
use rocket_dyn_templates::{Template, context};
use uuid::Uuid;
use tokio::sync::{oneshot::channel};

use crate::{gamerunner::{Message, Event, Outcome}, http::{session::NewSessionOutcome, models::NewGame}};

use super::{models::{GameSummary, GMView, IndexModel, PlayerView}, errors::Error, session::Session, metagame::Metagame};

#[get("/")]
pub async fn index(state: &State<Metagame<'_>>, session: Session) -> Result<Template, Error>
{

    let lock = state.game_details.read();
    let mut summaries = Vec::<GameSummary>::new();

    for (_id, details) in lock.iter()
    {
        summaries.push(GameSummary{ game_name: details.game_name.clone(), url: details.game_url.to_string(), gm: details.gm_id })
    }


    let model = IndexModel { player_handle: &session.handle_as_ref(), summaries  };


    return Ok(Template::render("index", model));
}

#[post("/game", data = "<new_game>")]
pub async fn create_game(state: &State<Metagame<'_>>, session: Session, new_game: Form<NewGame<'_>>) -> Result<Redirect, Error>
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
                    
                    state.new_game(game_id, session.player_id(), String::from(new_game.game_name), uri!(game_view(game_id)));
                    return Ok(Redirect::to(uri!(game_view(game_id))));
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
pub async fn game_view(id: Uuid, session: Session, state: &State<Metagame<'_>>) -> Result<Template, Error>
{
    let game_name = state.game_name(id);

    if game_name.is_none()
    {
        return Err(Error::NotFound(Template::render("error_pages/404", context!{})));
    }

    if state.validate_ownership( session.player_id(), id)
    {
        return Ok(Template::render("gm_view", GMView{game_id: id}));
    }
    else 
    {
        return Ok(Template::render("player_view", PlayerView{game_id: id, game_name: game_name.unwrap()}));
    }
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
{
    session.set_handle(String::from(submission.player_handle));
    Redirect::to(uri!("/"))
}