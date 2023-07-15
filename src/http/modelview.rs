use std::{sync::Arc, collections::HashMap};

use axum::{extract::State, http::{request, Request}, middleware::Next, response::Response};
use serde::Serialize;



pub struct ModelView
{
    view: String, 
    model: HashMap<String, String>,
}


pub async fn model_view_render<B>(
    State(state): State<Arc<super::state::State<'_>>>, 
    request: Request<B>,
    next: Next<B>
) -> Response
{
    let response = next.run(request).await;

    delegated_render(state, &response);

    return response;
}

fn delegated_render<B>
(
    state: Arc<super::state::State<'_>>, 
    response: &Response<B>,
)
{
    match response.extensions().get::<ModelView>()
    {
        None => {},
        Some(model_view) => {}
    //         if let Ok(html) = state.handlebars.render(model_view.view, model_view.model)
    //         {
    //             Response::builder().status(200)
    //             .header("Content-Type", "text/html")
    //             .body(html)
    //         }
    //         else
    //         {
    //             Response::builder().status(500).body(())
    //         }
    //     }
    }
}