use std::{sync::Arc, collections::HashMap};
use core::pin::Pin;
use core::future::Future;
use core::marker::Send;
use parking_lot::{RwLock, Mutex};
use rocket::{Request, request::{FromRequest, Outcome}};
use uuid::Uuid;

use super::errors::Error;

pub struct SessionData
{
    pub gm_of_games: Vec<Uuid>
}

pub struct Session
{
    session_data: Mutex<SessionData>
}

pub struct SessionMap
{
    sessions: RwLock<HashMap<Uuid, Session>>
}

pub fn new() -> SessionMap
{
    SessionMap { sessions: RwLock::new(HashMap::new()) }
}

impl SessionMap
{
    pub fn find_session(id: Uuid) -> Option<&Session>
    {

    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Session
{
    type Error = Error;

    async fn from_request(request: & 'r Request<'_>) ->  Outcome<Self,Self::Error>
    where 'r: 'async_trait, 'life0: 'async_trait,Self: 'async_trait 
    {
        match request.cookies().get("shadowrun_combat_session")
        {
            Some(session_cookie) => 
            {
                let session_id: Uuid;
                match Uuid::parse_str(session_cookie.value())
                {
                    Ok(temp) => { session_id = temp; },
                    Err(_) =>  {todo!()}
                }

                let mut map = request.rocket().state::<SessionMap>();
                

                match map.sessions.entry(session_id)
                {
                    std::collections::hash_map::Entry::Occupied(_) => 
                    {

                    },
                    std::collections::hash_map::Entry::Vacant(_) => todo!(),
                }
            },
            None => todo!(),
        };
        Outcome::Success(Session {gm_of_games: Vec::new()})
    }
}