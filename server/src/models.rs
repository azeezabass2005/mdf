use axum::{Json, http::StatusCode, response::{Response, IntoResponse}};
use serde::{Serialize, Deserialize};

use crate::{error::SemanticDataApiError, pdf_inference::reconstruct::ContentBlock};

#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticDataRequestBody {
    // TODO: The structure of the request coming from the frontend, it will probably contain the PDF
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticDataResponseBody {
    // TODO: The structure of the response body that will be sent to the frontend
    pdf_semantic_data: Vec<Vec<ContentBlock>>,
    file_name: String,
    message: String,
}

impl SemanticDataResponseBody {
    pub fn new(pdf_semantic_data: Vec<Vec<ContentBlock>>, file_name: String) -> Self {
        Self {
            pdf_semantic_data,
            file_name,
            message: String::from("Maldives generated successfully")
        }
    }
}

pub enum ApiResponse {
    Success(SemanticDataResponseBody),
    Error(SemanticDataApiError)
}

impl IntoResponse for ApiResponse {
    fn into_response(self) -> Response {
        match self {
            ApiResponse::Success(data) => (StatusCode::OK, Json(data)).into_response(),
            ApiResponse::Error(data) => data.into_response()
        }
    }
}