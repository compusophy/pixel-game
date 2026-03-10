use axum::Router;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    // serve wasm client files from static/ directory
    let app = Router::new()
        .fallback_service(ServeDir::new("static"));

    let addr = "0.0.0.0:3000";
    println!("server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
