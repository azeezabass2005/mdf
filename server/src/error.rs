use axum::{Json, http::StatusCode, response::IntoResponse};

/// Error enum for semantic data, the enums with argument string is the message/reason
pub enum SemanticDataApiError {
    BadRequest(String),
    SemanticInferenceError(String),
    InternalServerError,
}

impl IntoResponse for SemanticDataApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, Json(message)).into_response(),
            Self::SemanticInferenceError(message) => (StatusCode::UNPROCESSABLE_ENTITY, Json(message)).into_response(),
            Self::InternalServerError => (StatusCode::INTERNAL_SERVER_ERROR, Json("Something went wrong, Please try again later")).into_response()
        }
    }
}