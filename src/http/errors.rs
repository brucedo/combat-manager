
use rocket::{response::Responder};
use rocket_dyn_templates::Template;


#[derive(Responder, Debug)]
pub enum Error
{
    #[response(status=500)]
    InternalServerError(Template),
    #[response(status=403)]
    Forbidden(Template),
    #[response(status=404)]
    NotFound(Template),
}