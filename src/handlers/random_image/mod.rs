use std::{sync::Arc, time};
use axum::{body::Body, debug_handler, extract::State, response::Response};
use crate::errors;
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
// to multiple consumers, also leading to increased memory consumption.
//
// But we didn't want that. Because when the handler runs in parallel with several clients, they all would see the same random image.
//
// For those reasons the current way of handling the situation seems like one of the proper fits for the task
#[debug_handler]
#[deprecated(note = "not used much in production")]
pub async fn random_image(State(receiver): State<Arc<Mutex<mpsc::Receiver<Body>>>>) -> Result<Response<Body>, errors::AppError> {    
    let span = tracing::info_span!(target: "request", "handler_random_image");
    let _enter= span.enter();

    let bench = time::Instant::now();
    let res = Response::builder()
        .header("Cache-Control", "must-revalidate");

    let mut data = receiver.lock().await;
    let Some(body) = data.recv().await else {
        return Err(errors::AppError::SomeError { msg: "unfortunately, failed sampling an image".into() });
    };

    tracing::info!("sampling img took: {:?}", bench.elapsed());
    Ok(res.body(body)?)
}

pub async fn worker(sender: mpsc::Sender<Body>, images: Vec<String>) {
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
