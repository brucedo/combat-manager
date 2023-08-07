use log::debug;

use super::{PlayerId, GameId, dispatcher::Request, registry::GameRegistry};



pub fn authorize<'a, 'b>(player_id_opt: Option<GameId>, game_id_opt: Option<PlayerId>, request: Request, directory: &'b mut GameRegistry) -> Authority
{

    match (game_id_opt, player_id_opt)
    {
        (Some(game_id), Some(player_id)) => {
            debug!("Matching role for player id {} on game id {}", player_id, game_id);
            let resource_role = 
            if directory.is_gm(&player_id, &game_id)
            {
                Role::RoleGM(player_id, game_id)
            }
            else if directory.game_has_player(&game_id, &player_id)
            {
                Role::RolePlayer(player_id, game_id)
            }
            else if directory.is_registered(&player_id)
            {
                debug!("Player id {} is registered.", player_id);
                Role::RoleObserver(player_id, game_id)
            }
            else 
            {
                debug!("Player id {} is not registered.", player_id);
                Role::RoleUnregistered
            };

            // Authority {player_id: Some(player_id), game_id: Some(game_id), resource_role, request}
            Authority {resource_role, request }
        }, 
        (None, Some(player_id)) => {
            if directory.is_registered(&player_id)
            {
                debug!("Player ID {} is registered.", player_id);
                Authority {resource_role: Role::RoleRegistered(player_id), request}
            }
            else
            {
                debug!("Player ID {} is not registered.", player_id);
                Authority {resource_role: Role::RoleUnregistered, request}
            }
        }
        (_, None) =>
        {
            debug!("No player ID included - player unregistered.");
            Authority {resource_role: Role::RoleUnregistered, request}
        }
    }


    // Authority { player_id: todo!(), game_id: todo!(), resource_role: Role::RolePlayer, request: msg.msg }
}

#[derive(PartialEq)]
pub enum Role
{
    RoleGM(PlayerId, GameId),
    RolePlayer(PlayerId, GameId),
    RoleObserver(PlayerId, GameId),
    RoleRegistered(PlayerId),
    RoleUnregistered
}

pub struct Authority
{
    // player_id: Option<PlayerId>, 
    // game_id: Option<GameId>, 
    resource_role: Role,
    request: Request
}

impl Authority
{
    // pub fn player_id(&self) -> Option<PlayerId>
    // {
    //     self.player_id
    // }

    // pub fn game_id(&self) -> Option<GameId>
    // {
    //     self.game_id
    // }

    pub fn resource_role<'a>(&'a self) -> &'a Role
    {
        &self.resource_role
    }

    pub fn request<'a>(&'a self) -> &'a Request
    {
        &self.request
    }
}