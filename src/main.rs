use std::{sync::Arc, time};
use axum::{body::Body, debug_handler, extract::State, response::Response, Router};
use errors::AppError;
use rand::prelude::*;
use tokio_util::io::ReaderStream;
use tokio::sync::{mpsc, Mutex};

// Arc<Mutex<Receiver<T>>> is needed because a mpsc channel can have one receiver. By using Arc we share ownership.
// An Async Mutex is needed due to the primitives of Arc enforcing us otherwise taking &mut self would've failed on line 24.
//
// Async Mutexes are not blocking by literal meaning, as recv().await yields back control to the async runtime until the task can be resumed
// Ensuring everything works pretty solid and fast.
//
// Aside from those facts using Broadcast would've required us to implement clone on some sort of body as it propogates the same data by cloning the value
// to multiple consumers, also inheriting more memory consumption.
//
// But we didn't want that. Because when the handler runs in parallel with several clients, they all would see the same random image.
//
// For those reasons the current way of handling the situation seems like one of the proper fits for the task
#[debug_handler]
async fn random_image(State(receiver): State<Arc<Mutex<mpsc::Receiver<Body>>>>) -> Result<Response<Body>, errors::AppError> {    
    let bench = time::Instant::now();
    let res = Response::builder()
        .header("Cache-Control", "must-revalidate");

    let mut data = receiver.lock().await;
    let Some(body) = data.recv().await else {
        return Err(AppError::SomeError { msg: "unfortunately, failed sampling an image".into() });
    };

    println!("took: {:?}", bench.elapsed());
    Ok(res.body(body)?)
}

async fn worker(sender: mpsc::Sender<Body>, images: Vec<String>) {
    loop {
        let Some(image_path) = images.choose(&mut rand::rng()) else {
            continue
        };

        let Ok(file) = tokio::fs::File::open(image_path).await else {
            continue
        };

        let stream = Body::from_stream(ReaderStream::new(file));
        let _ = sender.send(stream).await;
    }
}

mod errors {
    use std::ffi::OsString;

    use axum::{http, response::IntoResponse};
    use serde::Serialize;
    use thiserror::Error;

    #[derive(Error, Debug, Serialize)]
    pub enum AppError {
        #[error("error: {msg}")]
        SomeError{msg: String}
    }

    impl IntoResponse for AppError {
        fn into_response(self) -> axum::response::Response {
            axum::Json::from(self).into_response()
        }
    }

    impl From<http::Error> for AppError {
        fn from(err: http::Error) -> Self {
            Self::SomeError { msg: err.to_string() }
        }
    }

    impl From<OsString> for AppError {
        fn from(err: OsString) -> Self {
            match err.into_string() {
                Ok(err) => Self::SomeError { msg: err },
                Err(_) => Self::SomeError { msg: "failed transforming to `OsString`".into() },
            }
        }
    }

    impl From<std::io::Error> for AppError {
        fn from(err: std::io::Error) -> Self {
            Self::SomeError { msg: err.to_string() }
        }
    }
}

#[tokio::main]
async fn main() {
    let mut images: Vec<String> = vec![];
    let wdir = std::env::current_dir()
        .map(|dir| dir.join("images"));

    if let Ok(wdir) = wdir {
        let mut walk_dir = tokio::fs::read_dir(wdir)
            .await.unwrap();

        while let Ok(Some(entry)) = walk_dir.next_entry().await {
            images.push(entry.path().display().to_string());
        }
    }

    let (sender, receiver) = mpsc::channel::<Body>(20);
    for _ in 0..20 {
        let cloned_sender = sender.clone();
        let cloned_img = images.clone();

        tokio::spawn( async move {
            worker(cloned_sender, cloned_img).await
        });
    }

    let app = Router::new()
        .fallback(random_image)
        .with_state(Arc::new(Mutex::new(receiver)));        
    let listener = tokio::net::TcpListener::bind("0.0.0.0:2000")
        .await
        .unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
}