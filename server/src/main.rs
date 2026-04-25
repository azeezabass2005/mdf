use axum::{Router, http::{HeaderValue, Method}, routing::post};
use tower_http::cors::{Any, CorsLayer};

use crate::api::generate_semantic_data;

pub mod pdf_inference;
pub mod error;
pub mod models;
pub mod api;

#[tokio::main]
async fn main() {
    println!("MDF - The Maldives for PDFs");
    let cors = CorsLayer::new()
        .allow_methods([Method::POST, Method::GET, Method::PATCH])
        .allow_origin([
            "http://localhost:5173".parse::<HeaderValue>().unwrap(),
            "https://mdf.thefola.dev".parse::<HeaderValue>().unwrap()
        ])
        .allow_headers(Any);

    let router = Router::new()
        .route("/infer_semantics", post(generate_semantic_data))
        .layer(cors);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000").await.unwrap();
    println!("Listening on port: {:?}", listener.local_addr());
    axum::serve(listener, router).await.unwrap();
}
