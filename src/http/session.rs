use std::{collections::HashMap, sync::Arc};
use log::debug;
use parking_lot::{RwLock, Mutex};
use rocket::{Request, request::{FromRequest, Outcome, self}, http::Cookie, time::{OffsetDateTime, Duration}};
use uuid::Uuid;

pub struct SessionData
{
    pub gm_of_games: Vec<Uuid>,
    pub handle: Arc<String>,
    pub player_id: Arc<Uuid>,
    pub game_to_character: HashMap<Uuid, Uuid>
}

impl SessionData
{
    pub fn new() -> SessionData
    {
        SessionData 
        { 
            gm_of_games: Vec::new(), 
            handle: Arc::new(String::from("__none__")), 
            player_id: Arc::new(Uuid::new_v4()),
            game_to_character: HashMap::new(), 
        }
    }
}

pub struct Session
{
    session_data: Arc<Mutex<SessionData>>
}

impl Session
{
    pub fn new() -> Session
    {
        Session { session_data: Arc::new(Mutex::new(SessionData::new())) }
    }

    pub fn clone(&self) -> Session
    {
        Session { session_data: self.session_data.clone()}
    }

    pub fn set_handle(&self, name: String)
    {
        let mut data = self.session_data.lock();
        data.handle = Arc::new(name);
    }

    pub fn handle_as_ref(&self) -> Arc<String>
    {
        let data = self.session_data.lock();
        let temp = data.handle.clone();
        return temp;
    }

    pub fn store_new_game(&self, game_id: Uuid)
    {
        let mut data = self.session_data.lock();
        data.gm_of_games.push(game_id);
    }

    pub fn player_id(&self) -> Uuid
    {
        (*self.session_data.lock().player_id).clone()
    }

    pub fn add_pc(&self, game_id: Uuid, char_id: Uuid)
    {
        let mut data = self.session_data.lock();

        data.game_to_character.insert(game_id, char_id);
    }

    pub fn has_character_for(&self, game_id: Uuid) -> bool
    {
        self.session_data.lock().game_to_character.contains_key(&game_id)
    }

    pub fn character_id(&self, game_id: Uuid) -> Option<Uuid>
    {
        let data = self.session_data.lock();
        match data.game_to_character.get(&game_id) {
            Some(id) => Some(id.clone()),
            None => None,
        }
    }
}

pub struct SessionMap
{
    sessions: RwLock<HashMap<Uuid, Session>>
}



impl SessionMap
{
    pub fn new() -> SessionMap
    {
        SessionMap { sessions: RwLock::new(HashMap::new()) }
    }

    pub fn find_session(&self, id: Uuid) -> Option<Session>
    {

        if let Some(temp) = self.sessions.read().get(&id)
        {
            return Some(temp.clone());
        }
        else
        {
            None
        }
    }

    pub fn add_session(&self, id: Uuid, session: Session)
    {
        self.sessions.write().insert(id, session);
    }

    pub fn drop_session(&self, id: Uuid)
    {
        self.sessions.write().remove(&id);
    }
}

#[derive(Debug)]
pub struct NoSession
{

}

#[derive(Debug)]
pub enum NewSessionOutcome
{
    New,
    Exists,
    Expired,

}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Session
{
    type Error = NoSession;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error>
    // where 'r: 'async_trait, 'life0: 'async_trait,Self: 'async_trait 
    {
        debug!("Session guard firing.");
        match request.cookies().get_pending("shadowrun_combat_session")
        {
            Some(session_cookie) => 
            {
                debug!("Session cookie lookup succeeded.");
                let session_id: Uuid;
                match Uuid::parse_str(session_cookie.value())
                {
                    Ok(temp) => 
                    { 
                        session_id = temp; 
                        debug!("Session ID parsed into UUID.");
                    },
                    Err(_) =>  
                    {
                        debug!("Session ID is not a valid UUID form.");
                        return Outcome::Forward(())
                    }
                }

                let map = request.rocket().state::<SessionMap>().unwrap_or_else(|| panic!());

                match map.find_session(session_id)
                {
                    Some(session) => 
                    {
                        debug!("Session UUID maps to a valid Session object");
                        return Outcome::Success(session);
                    },
                    None => 
                    {
                        debug!("Session UUID does NOT map to a valid Session object - throwing.");
                        return Outcome::Forward(())
                    },
                }
            },
            None => 
            {
                debug!("No session cookie present in jar.");
                return Outcome::Forward(())
            },
        };
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for NewSessionOutcome
{
    type Error = NewSessionOutcome;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error>
    {
        debug!("Starting NewSessionOutcome guard.");
        let new_session_id = Uuid::new_v4();
        let response: Outcome<Self, Self::Error>;
        match request.cookies().get("shadowrun_combat_session")
        {
            Some(session_cookie) =>
            {
                match Uuid::parse_str(session_cookie.value())
                {
                    Ok(session_id) => 
                    {
                        let map = request.rocket().state::<SessionMap>().unwrap_or_else(|| panic!());
                        map.drop_session(session_id);

                        response = Outcome::Success(NewSessionOutcome::Exists);
                    },
                    Err(_) => 
                    {
                        response = Outcome::Success(NewSessionOutcome::New)
                    }
                }
            }
            None =>
            {
                response = Outcome::Success(NewSessionOutcome::New);
            }
        }

        let new_session = Session::new();
        let map = request.rocket().state::<SessionMap>().unwrap_or_else(|| panic!());
        map.add_session(new_session_id, new_session);
        let session_cookie = Cookie::build("shadowrun_combat_session", new_session_id.to_string())
            .expires(OffsetDateTime::now_utc().saturating_add(Duration::DAY))
            .finish();
        request.cookies().add(session_cookie);

        debug!("Finishing new session with value {}", response);

        return response;
    }
}