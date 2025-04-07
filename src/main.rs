use std::sync::Arc;
use axum::{body::Body, routing::get, Router};
use random_img_server::handlers::cat_image;
use tokio::sync::{mpsc, Mutex};
use tower_http::{classify::StatusInRangeAsFailures, trace::TraceLayer};
use tracing::{info_span, Level};
use tracing_subscriber::fmt::time::ChronoLocal;
#[expect(deprecated)]
use random_img_server::handlers::{worker, random_image};


#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();

    tracing_subscriber::fmt()
        .with_timer(ChronoLocal::new("%v [%T]".into()))
        .with_max_level(Level::INFO)
        .init();
    
    let mut images: Vec<String> = vec![];
    let walk_dir = std::env::current_dir()
        .map(|dir| dir.join("images"))
        .unwrap();

    let mut walk_dir = tokio::fs::read_dir(walk_dir).await.unwrap();
    while let Ok(Some(entry)) = walk_dir.next_entry().await {
        images.push(entry.path().display().to_string());
    }

    let (sender, receiver) = mpsc::channel::<Body>(20);
    for _ in 0..20 {
        let cloned_sender = sender.clone();
        let cloned_img = images.clone();

        tokio::spawn( async move {
            worker(cloned_sender, cloned_img).await
        });
    }

    let state = Arc::new(Mutex::new(receiver));   
    #[expect(deprecated)]
    let app = Router::new()
        .route("/", get(random_image))
        .route("/cat", get(cat_image))
        .layer(
            TraceLayer::new(StatusInRangeAsFailures::new(400..=599).into_make_classifier())
                .make_span_with(|request: &axum::http::Request<axum::body::Body>| {
                    let matched_path = request
                        .extensions()
                        .get::<axum::extract::MatchedPath>()
                        .map(axum::extract::MatchedPath::as_str);

                    info_span!(
                        "request",
                        method = ?request.method(),
                        path = matched_path
                    )
                })
        )
        .fallback(axum::response::Redirect::to("/"))
        .with_state(state);        
    
    axum::serve(tokio::net::TcpListener::bind("0.0.0.0:2000").await.unwrap(), app)
        .await
        .unwrap();
}