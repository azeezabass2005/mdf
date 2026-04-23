use axum::{Router, routing::post};

use crate::api::generate_semantic_data;

pub mod pdf_inference;
pub mod error;
pub mod models;
pub mod api;

#[tokio::main]
async fn main() {
    println!("MDF - The Maldives for PDFs");
    let router = Router::new()
        .route("/infer_semantics", post(generate_semantic_data));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000").await.unwrap();
    println!("Listening on port: {:?}", listener.local_addr());
    axum::serve(listener, router).await.unwrap();
}
