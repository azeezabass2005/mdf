use crate::parser::extract_pdf_text;

pub mod parser;
pub mod error;

#[tokio::main]
async fn main() {
    println!("MDF - The Maldives for PDFs");
    let extraction_result = extract_pdf_text();
    match extraction_result {
        Ok(_) => println!("Text extracted from pdf successfully"),
        Err(error) => {
            println!("This is the error that occurred during text extraction: {:?}", error)
        }
    }
}
