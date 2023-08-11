use std::{sync::Arc, collections::HashMap};

use axum::{extract::State, http::Request, middleware::Next, response::Response, body::Bytes};
use axum_macros::debug_handler;
use log::{debug, error};
use erased_serde::Serialize;


pub struct ModelView
{
    pub view: String, 
    pub model: HashMap<String, String>,
}

pub struct ModelView2
// where T: Serialize + Send + Sync + 'static
{
    pub view: &'static str,
    pub model: Box<dyn Serialize + Send + Sync + 'static>
}

pub struct StaticView
{
    pub view: String
}


pub async fn static_file_render<B>
(
    State(state): State<Arc<super::state::State<'_>>>,
    request: Request<B>,
    next: Next<B>
) -> Response
{
    debug!("static_file_render invoked pre-response");

    let response = next.run(request).await;

    debug!("static_file_render invoked post-response");

    match response.extensions().get::<StaticView>()
    {
        None => {response},
        Some(static_view) => {
            if let (Some(response_body), Some(mime_type)) = 
                (state.statics.get_resource(&static_view.view), state.statics.get_mime(&static_view.view))
            {
                // axum::body::boxed(axum::body::Full::from(owned_body));
                Response::builder().header("Content-Type", mime_type)
                    .body( axum::body::boxed(axum::body::Full::from(axum::body::Bytes::from(response_body))))
                    .unwrap()
            }
            else
            {
                let mut error = HashMap::new();
                error.insert(String::from("error"), String::from(format!("No static resource named {} found.", &static_view.view)));
                Response::builder().extension(ModelView{view: String::from("500"), model: error}).body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap()
            }
        }
    }

    // return response;
}

pub async fn model_view_render<B>(
    State(state): State<Arc<super::state::State<'_>>>, 
    request: Request<B>,
    next: Next<B>
) -> Response
{
    debug!("model_view_render invoked pre-response");

    let response = next.run(request).await;

    debug!("model_view_render invoked post-response");

    match (response.extensions().get::<ModelView>(), response.extensions().get::<ModelView2>())
    {
        (None, None) => {response},
        (Some(model_view), _) => {
            debug!("There is a ModelView extension attached to this request.  Attempting to process and render.");
            if let Ok(html) = state.handlebars.render(&model_view.view, &model_view.model)
            {
                Response::builder().status(200)
                .header("Content-Type", "text/html; charset=UTF-8")
                .body(axum::body::boxed(axum::body::Full::from(html))).unwrap()
            }
            else
            {
                Response::builder().status(500).body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap()
            }
        },
        (_, Some(model_view2)) => {
            debug!("There is a ModelView2 extension attached to this request.  Attempting to process and render.");
            match state.handlebars.render(model_view2.view, &model_view2.model)
            {
                Ok(html) => {
                    debug!("The render processed perfectly, generating response with status and body now.");
                    Response::builder().status(200)
                    .header("Content-Type", "text/html; charset=UTF-8")
                    .body(axum::body::boxed(axum::body::Full::from(html))).unwrap()
                },
                Err(e) => {
                    error!("The render failed.  Reason: {}", e.desc);
                    Response::builder().status(500).body(axum::body::boxed(axum::body::Empty::<Bytes>::new())).unwrap()
                }
            }
        }
    }

    // return response;
}
