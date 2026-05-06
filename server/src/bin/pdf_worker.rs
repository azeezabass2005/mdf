use std::io::Read;
use std::process::ExitCode;

use pdf_maldives_be::pdf_inference::{
    init_pdfium,
    reconstruct::{reconstruct_page, ContentBlock},
};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: pdf_worker <worker_index> <n_workers>");
        return ExitCode::from(2);
    }

    let worker_index: usize = match args[1].parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("invalid worker_index");
            return ExitCode::from(2);
        }
    };
    let n_workers: usize = match args[2].parse() {
        Ok(n) if n > 0 => n,
        _ => {
            eprintln!("invalid n_workers");
            return ExitCode::from(2);
        }
    };

    let mut pdf_bytes = Vec::new();
    if let Err(e) = std::io::stdin().read_to_end(&mut pdf_bytes) {
        eprintln!("failed to read stdin: {}", e);
        return ExitCode::from(3);
    }

    let pdfium = init_pdfium();
    let document = match pdfium.load_pdf_from_byte_slice(&pdf_bytes, None) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("failed to load pdf: {:?}", e);
            return ExitCode::from(4);
        }
    };
    let page_count = document.pages().len() as usize;

    let mut results: Vec<(usize, Vec<ContentBlock>)> = Vec::new();
    let mut idx = worker_index;
    while idx < page_count {
        let page = match document.pages().get(idx as i32) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("failed to get page {}: {:?}", idx, e);
                return ExitCode::from(5);
            }
        };
        let blocks = reconstruct_page(&page);
        results.push((idx, blocks));
        idx += n_workers;
    }

    let stdout = std::io::stdout();
    let handle = stdout.lock();
    if let Err(e) = serde_json::to_writer(handle, &results) {
        eprintln!("failed to serialize: {}", e);
        return ExitCode::from(6);
    }

    ExitCode::SUCCESS
}
