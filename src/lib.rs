pub mod handlers;
pub mod validator;
pub mod errors {
    use std::ffi::OsString;

    use axum::{http::{self, StatusCode}, response::IntoResponse};
    use serde::Serialize;
    use thiserror::Error;

    #[derive(Error, Debug, Serialize)]
    pub enum AppError {
        #[error("error: {msg}")]
        SomeError{msg: String},

        #[error(transparent)]
        ValidationError(#[from] validator::ValidationErrors),
    }

    impl IntoResponse for AppError {
        fn into_response(self) -> axum::response::Response {
            (StatusCode::BAD_REQUEST, axum::Json::from(self)).into_response()
        }
    }

    impl<T> From<Box<T>> for AppError
    where
        T: std::error::Error
    {
        fn from(err: Box<T>) -> Self {
            AppError::SomeError { msg: err.to_string() }
        }
    }

    impl From<http::Error> for AppError {
        fn from(err: http::Error) -> Self {
            Self::SomeError { msg: err.to_string() }
        }
    }

    impl From<dotenvy::Error> for AppError {
        fn from(err: dotenvy::Error) -> Self {
            Self::SomeError { msg: err.to_string() }
        }
    }

    impl From<reqwest::Error> for AppError {
        fn from(err: reqwest::Error) -> Self {
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