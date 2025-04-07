use std::future::Future;
use axum::{extract::FromRequestParts, http::request::Parts};
use serde::{de::DeserializeOwned, Deserialize};
use validator::Validate;
use crate::errors;

pub struct FormValidator<T: Validate + DeserializeOwned>(pub T);

#[derive(Debug, Validate, Deserialize)]
pub struct QueryOpts {
    #[validate(length(min = 3, max = 30))]
    #[serde(rename = "q")]
    pub query: String,
}

impl<S> FromRequestParts<S> for FormValidator<QueryOpts>
where 
    S: Send + Sync
    
{
    type Rejection = errors::AppError;

    fn from_request_parts(parts: &mut Parts, _: &S) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async {
            let Some(path) = parts.uri.query() else {
                return Err(errors::AppError::SomeError { msg: "failed parsing query".into() })
            };
    
            match path.contains("q=").then(|| path) {
                Some(query) => {
                    let q = QueryOpts{query: query.into()};
                    q.validate()?;

                    let validator = FormValidator(q);   
                    Ok(validator)
                },
                None => Err(errors::AppError::SomeError { msg: "only qs 'q' is accepted".into() }),
            }
        }
        
    }
}