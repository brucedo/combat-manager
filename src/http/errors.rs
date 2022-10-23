
use rocket::{response::Responder, http::Status};
use rocket_dyn_templates::Template;


#[derive(Responder, Debug)]
pub enum Error
{
    #[response(status=500)]
    InternalServerError(Template),
    #[response(status=403)]
    Forbidden(Template),
}