use super::{PlayerId, GameId, dispatcher::Request, registry::GameRegistry};



pub fn authorize(request: Request, directory: &GameRegistry) -> Authority
{

    Authority { player_id: todo!(), game_id: todo!(), resource_role: Role::RolePlayer, request }
}

enum Role
{
    RoleGM,
    RolePlayer,
}

pub struct Authority<'a>
{
    player_id: &'a PlayerId, 
    game_id: &'a GameId, 
    resource_role: Role,
    request: Request
}