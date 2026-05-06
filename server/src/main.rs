use axum::{Router, http::{HeaderValue, Method}, routing::post};
use pdfium_render::prelude::PdfPage;
use tower_http::cors::{Any, CorsLayer};

use pdf_maldives_be::api::generate_semantic_data;

#[tokio::main]
async fn main() {
    println!("MDF - The Maldives for PDFs");

    assert_send::<PdfPage>();


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

pub fn assert_send<T: Send>() {
    // The function does nothing
}