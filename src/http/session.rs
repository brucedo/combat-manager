use std::{collections::HashMap, sync::Arc};
use parking_lot::{RwLock, Mutex};
use rocket::{Request, request::{FromRequest, Outcome, self}};
use uuid::Uuid;

use super::errors::Error;

pub struct SessionData
{
    pub gm_of_games: Vec<Uuid>
}

pub struct Session
{
    session_data: Arc<Mutex<SessionData>>
}

impl Session
{
    pub fn new() -> Session
    {
        Session { session_data: Arc::new(Mutex::new(SessionData {gm_of_games: Vec::new()})) }
    }

    pub fn clone(&self) -> Session
    {
        Session { session_data: self.session_data.clone()}
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

    pub fn add_session(&mut self, id: Uuid, session: Session)
    {
        self.sessions.write().insert(id, session);
    }
}

#[derive(Debug)]
pub struct NoSession
{

}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Session
{
    type Error = NoSession;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error>
    // where 'r: 'async_trait, 'life0: 'async_trait,Self: 'async_trait 
    {
        match request.cookies().get("shadowrun_combat_session")
        {
            Some(session_cookie) => 
            {
                let session_id: Uuid;
                match Uuid::parse_str(session_cookie.value())
                {
                    Ok(temp) => { session_id = temp; },
                    Err(_) =>  {session_id = Uuid::new_v4()}
                }

                let map = request.rocket().state::<SessionMap>().unwrap_or_else(|| panic!());

                match map.find_session(session_id)
                {
                    Some(session) => {return Outcome::Success(session);},
                    None => {return Outcome::Forward(())},
                }
            },
            None => return Outcome::Forward(()),
        };
        // Outcome::Success(Session::new())
    }
}