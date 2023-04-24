use super::{PlayerId, GameId, dispatcher::{Request, Message}, registry::GameRegistry};



pub fn authorize<'a, 'b>(game_id: Option<GameId>, player_id: Option<PlayerId>, request: Request, directory: &'b GameRegistry) -> Authority
{

    match (game_id, player_id)
    {
        (_, None) | (None, _) => {
            Authority {player_id: player_id, game_id: game_id, resource_role: Role::RoleObserver, request}
        },
        (Some(game_id), Some(player_id)) => {
            let resource_role: Role = 
            if directory.is_gm(&player_id, &game_id)
            {
                Role::RoleGM
            }
            else if directory.game_has_player(&game_id, &player_id)
            {
                Role::RolePlayer
            }
            else 
            {
                Role::RoleObserver
            };

            Authority {player_id: Some(player_id), game_id: Some(game_id), resource_role, request}
        }
    }


    // Authority { player_id: todo!(), game_id: todo!(), resource_role: Role::RolePlayer, request: msg.msg }
}

#[derive(PartialEq)]
pub enum Role
{
    RoleGM,
    RolePlayer,
    RoleObserver,
}

pub struct Authority
{
    player_id: Option<PlayerId>, 
    game_id: Option<GameId>, 
    resource_role: Role,
    request: Request
}

impl Authority 
{
    pub fn player_id(&self) -> Option<PlayerId>
    {
        self.player_id
    }

    pub fn game_id(&self) -> Option<GameId>
    {
        self.game_id
    }

    pub fn resource_role<'a>(&'a self) -> &'a Role
    {
        &self.resource_role
    }

    pub fn request<'a>(&'a self) -> Request
    {
        self.request
    }
}