use axum::{extract::Multipart, response::IntoResponse};

use crate::{error::SemanticDataApiError, models::{ApiResponse, SemanticDataResponseBody}, pdf_inference::infer_pdf_semantics};

pub async fn generate_semantic_data(mut multipart: Multipart) -> impl IntoResponse {
    let field = match multipart.next_field().await {
        Ok(Some(field)) => field,
        Ok(None) => {
            return ApiResponse::Error(SemanticDataApiError::BadRequest(
                "No file provided in the request".to_string(),
            ));
        }
        Err(error) => {
            return ApiResponse::Error(SemanticDataApiError::BadRequest(format!(
                "Could not read the uploaded file: {error}"
            )));
        }
    };

    let file_name = field
        .file_name()
        .map(|name| name.to_string())
        .unwrap_or_else(|| "document.pdf".to_string());

    let data = match field.bytes().await {
        Ok(data) => data,
        Err(error) => {
            return ApiResponse::Error(SemanticDataApiError::BadRequest(format!(
                "Could not read the uploaded file: {error}"
            )));
        }
    };

    match infer_pdf_semantics(&data) {
        Ok(inference_data) => {
            ApiResponse::Success(SemanticDataResponseBody::new(inference_data, file_name))
        }
        Err(error) => {
            ApiResponse::Error(SemanticDataApiError::SemanticInferenceError(error.to_string()))
        }
    }
}
