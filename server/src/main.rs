use crate::parser::extract_structured_text;

pub mod parser;
pub mod error;

#[tokio::main]
async fn main() {
    println!("MDF - The Maldives for PDFs");
    let extraction_result = extract_structured_text();
    match extraction_result {
        Ok(_) => println!("\nText extracted and reconstructed successfully"),
        Err(error) => {
            println!("This is the error that occurred during text extraction: {:?}", error)
        }
    }
}
