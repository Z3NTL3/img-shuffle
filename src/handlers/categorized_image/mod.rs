use axum::{body::Body, debug_handler,response::{IntoResponse, Response}};
use serde::Deserialize;
use crate::{errors, validator::{QueryOpts, FormValidator}};

#[derive(Deserialize)]
struct ImageSamples {
    hits: Vec<Hit>
}

#[derive(Deserialize)]
struct Hit {
    #[serde(rename = "largeImageURL")]
    large_img: String
}

#[debug_handler]
pub async fn cat_image(FormValidator(opts): FormValidator<QueryOpts>) -> Result<Response<Body>, errors::AppError> {  
    let samples = reqwest::get(
        format!("https://pixabay.com/api/?key={}&{}&image_type=photo", dotenvy::var("API_KEY")?, opts.query)
    )
        .await?
        .json::<ImageSamples>()
        .await?;

    let Some(sample) = samples.hits.get(0) else {
        return Err(errors::AppError::SomeError { msg: "failed samplimg image from PixaBay API".into() })
    };
    
    // who cares its a school project
    let res = reqwest::get(sample.large_img.clone())
        .await?
        .bytes_stream();

    Ok(Body::from_stream(res).into_response())
}