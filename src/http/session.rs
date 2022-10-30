use std::{collections::HashMap, sync::Arc};
use parking_lot::{RwLock, Mutex};
use rocket::{Request, request::{FromRequest, Outcome, self}, http::Cookie, time::{OffsetDateTime, Duration}};
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
        match request.cookies().get("shadowrun_combat_session")
        {
            Some(session_cookie) => 
            {
                let session_id: Uuid;
                match Uuid::parse_str(session_cookie.value())
                {
                    Ok(temp) => { session_id = temp; },
                    Err(_) =>  {return Outcome::Forward(())}
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
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for NewSessionOutcome
{
    type Error = NewSessionOutcome;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error>
    {
        let new_session_id = Uuid::new_v4();
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

                        let new_session = Session::new();
                        let map = request.rocket().state::<SessionMap>().unwrap_or_else(|| panic!());
                        map.add_session(new_session_id, new_session);
                        let session_cookie = Cookie::build("shadowrun_combat_session", new_session_id.to_string())
                            .expires(OffsetDateTime::now_utc().saturating_add(Duration::DAY))
                            .finish();
                        request.cookies().add(session_cookie);

                        return Outcome::Success(NewSessionOutcome::Exists);
                    },
                    Err(_) => 
                    {
                        let new_session = Session::new();
                        let map = request.rocket().state::<SessionMap>().unwrap_or_else(|| panic!());
                        map.add_session(new_session_id, new_session);
                        let session_cookie = Cookie::build("shadowrun_combat_session", new_session_id.to_string())
                            .expires(OffsetDateTime::now_utc().saturating_add(Duration::DAY))
                            .finish();
                        request.cookies().add(session_cookie);
                        return Outcome::Success(NewSessionOutcome::New);                        
                    }
                }
            }
            None =>
            {
                let new_session = Session::new();
                let map = request.rocket().state::<SessionMap>().unwrap_or_else(|| panic!());
                map.add_session(new_session_id, new_session);
                let session_cookie = Cookie::build("shadowrun_combat_session", new_session_id.to_string())
                    .expires(OffsetDateTime::now_utc().saturating_add(Duration::DAY))
                    .finish();
                request.cookies().add(session_cookie);
                return Outcome::Success(NewSessionOutcome::New);  
            }
        }
    }
}