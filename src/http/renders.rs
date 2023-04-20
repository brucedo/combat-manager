
use log::debug;
use rocket::{get, post, State, response::Redirect, uri, form::{FromForm, Form}};
use rocket_dyn_templates::{Template, context};
use uuid::Uuid;
use tokio::sync::{oneshot::channel, mpsc::Sender};

use crate::{gamerunner::dispatcher::{Message, Request, Outcome}, http::{session::NewSessionOutcome, models::NewGame}, tracker::character::Character};

use super::{models::{GameSummary, GMView, IndexModel, PlayerView, SimpleCharacterView, NewCharacter}, errors::Error, session::Session, metagame::Metagame};

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

    let response = send_and_recv(Uuid::new_v4(), Request::New, my_sender).await?;

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
        build_gm_view(id, &session, state).await
        
    }
    else 
    {
        build_player_view(id, &session, state).await
    } 

}

async fn build_player_view(game_id: Uuid, session: &Session, state: &State<Metagame<'_>>) -> Result<Template, Error>
{
    let game_name = state.game_name(game_id).unwrap_or(String::from(""));
    let view: PlayerView;

    if session.has_character_for(game_id)
    {
        match send_and_recv(game_id, Request::GetCharacter(session.character_id(game_id).unwrap()), state.game_runner_pipe.clone()).await?
        {
            Outcome::Found(char) => 
            {
                view = PlayerView {player_handle: session.handle_as_ref(), game_id, game_name, character_state: Some(SimpleCharacterView::from(char.unwrap().as_ref()))};
            }
            _ => {
                let err = "Boy howdy, something really went south here.  We received a completely unexpected message type from the GameRunner for creating a game.";
                return Err(Error::InternalServerError(Template::render("error_pages/500", context! {action_name: "create a new game", error: err})));
            }
        }
    }
    else
    {
        view = PlayerView {player_handle: session.handle_as_ref(), game_id, game_name, character_state: None };
    }

    // let view = PlayerView {game_id, game_name, character_state: None };

    Ok(Template::render("player_view", view))
}

async fn build_gm_view(game_id: Uuid, sesion: &Session, state: &State<Metagame<'_>>) -> Result<Template, Error>
{
    let outcome = send_and_recv(game_id, Request::GetPcCast, state.game_runner_pipe.clone()).await?;
    let mut pcs: Vec<SimpleCharacterView>;
    let mut npcs: Vec<SimpleCharacterView>;
    let game_name = state.game_name(game_id).unwrap_or(String::from(""));

    match outcome
    {
        Outcome::CastList(cast) => 
        {
            pcs = Vec::with_capacity(cast.len());
            debug!("Converting Character to SimpleCharacterView for {} records", cast.len());
            for member in cast
            {
                pcs.push(SimpleCharacterView::from(member.as_ref()));
            }
        }
        _ => 
        {
            let err = "Boy howdy, something really went south here.  We received a completely unexpected message type from the GameRunner for creating a game.";
            return Err(Error::InternalServerError(Template::render("error_pages/500", context! {action_name: "create a new game", error: err})));
        }
    }

    let outcome = send_and_recv(game_id, Request::GetNpcCast, state.game_runner_pipe.clone()).await?;
    
    match outcome
    {
        Outcome::CastList(cast) => 
        {
            npcs = Vec::with_capacity(cast.len());
            for member in cast
            {
                npcs.push(SimpleCharacterView::from(member.as_ref()));
            }
        }
        _ => 
        {
            let err = "Boy howdy, something really went south here.  We received a completely unexpected message type from the GameRunner for creating a game.";
            return Err(Error::InternalServerError(Template::render("error_pages/500", context! {action_name: "create a new game", error: err})));
        }
    }

    return Ok(Template::render("gm_view", GMView { game_id, pcs, npcs }));
}

#[post("/game/<id>/add_npc", data="<npc>")]
pub async fn add_npc(id: Uuid, session: Session, state: &State<Metagame<'_>>, npc: Form<NewCharacter<'_>>) -> Result<Redirect, Error>
{

    if !state.validate_ownership(session.player_id(), id)
    {
        // TODO: build a 403 tsk tsk tsk kinda
    }

    let character = Character::from(npc.into_inner());
    
    let result = send_and_recv(id, Request::AddCharacter(character), state.game_runner_pipe.clone()).await?;

    match result
    {
        Outcome::CharacterAdded(_) => 
        {
            // return Ok(Template::render("added", context!{game_id: id}));
            return Ok(Redirect::to(uri!(game_view(id))));
        },
        Outcome::Error(err) => {return Err(Error::InternalServerError(Template::render("500", context! {action_name: "create a character", error: err.message})))},
        _ => {return Err(Error::InternalServerError(Template::render("500", context! {action_name: "create a character", error: "The Game replied with an unexpected message."})))}
    }
}

#[post("/game/<id>/add_pc", data="<pc>")]
pub async fn add_pc(id: Uuid, session: Session, state: &State<Metagame<'_>>, pc: Form<NewCharacter<'_>>) -> Result<Redirect, Error>
{
    let character = Character::from(pc.into_inner());

    let result = send_and_recv(id, Request::AddCharacter(character), state.game_runner_pipe.clone()).await?;
    
    match result
    {
        Outcome::CharacterAdded((_, char_id)) => 
        {
            session.add_pc(id, char_id);
            return Ok(Redirect::to(uri!(game_view(id))));
        },
        Outcome::Error(err) => {return Err(Error::InternalServerError(Template::render("500", context! {action_name: "create a character", error: err.message})))},
        _ => {return Err(Error::InternalServerError(Template::render("500", context! {action_name: "create a character", error: "The Game replied with an unexpected message."})))}
    }
    
    
}

#[get("/<_..>", rank = 11)]
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

async fn send_and_recv(game_id: Uuid, body: Request, sender: Sender<Message>) -> Result<Outcome, Error>
{
    let (their_sender, my_receiver) = channel::<Outcome>();
    let msg = Message { game_id, reply_channel: their_sender, msg: body };
    if let Err(_err) = sender.send(msg).await
    {
        return Err(Error::InternalServerError(Template::render("500", context! {action_name: "create a character", error: "The game runner closed its channel."})));
    }

    match my_receiver.await 
    {
        Ok(outcome) => Ok(outcome),
        Err(_err) => 
            Err(Error::InternalServerError(Template::render("500", context! {action_name: "create a character", error: "The reply channel was closed."}))),
    }
}