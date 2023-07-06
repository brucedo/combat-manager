use rocket::get;
use uuid::Uuid;

use rocket::response::stream::{Event, EventStream};
use rocket::tokio::time::{self, Duration};

#[get("/<group_id>")]
pub fn start_message_stream(group_id: Uuid) -> EventStream![] {
    EventStream! {
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            yield Event::data("ping");
            interval.tick().await;
        }
    }
}
