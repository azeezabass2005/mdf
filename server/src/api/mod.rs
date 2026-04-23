use axum::{extract::Multipart,  response::IntoResponse};

use crate::{error::SemanticDataApiError, models::{ApiResponse, SemanticDataResponseBody}, pdf_inference::infer_pdf_semantics};

pub async fn generate_semantic_data(mut multipart: Multipart) -> impl IntoResponse {
    let field = multipart.next_field().await.unwrap();
        
    match field {
        Some(field) => {
            let file_name = field.file_name().unwrap().to_string();
            let data = field.bytes().await.unwrap();
            let inference_result = infer_pdf_semantics(&data);
            match inference_result {
                Ok(inference_data) => {
                    return ApiResponse::Success(SemanticDataResponseBody::new(inference_data, file_name));
                },
                Err(error) => {
                    return ApiResponse::Error(SemanticDataApiError::SemanticInferenceError(error.to_string()));
                }
            }

            // TODO: I won't use any DB, I will just allow the frontend to store in index db and allow the data available for user offline
            // TODO: I can also allow something like export your files to another device, to allow the user to use both their laptop and phone
            // I will make the above feature available offline over wifi
        },
        None => {
            return ApiResponse::Error(SemanticDataApiError::InternalServerError);
        }
    }
}