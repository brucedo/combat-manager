use poem::{Route, get, handler, listener::TcpListener, Server};


pub async fn launch_server()
{
    let routes = Route::new().at("/", get(bootstrap));
    Server::new(TcpListener::bind("localhost:8080")).run(routes).await;
}

#[handler]
pub fn bootstrap()
{

}