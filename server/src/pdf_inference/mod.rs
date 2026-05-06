use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use pdfium_render::prelude::*;
use crate::pdf_inference::reconstruct::ContentBlock;

pub mod reconstruct;

static PDFIUM_BINDINGS: OnceLock<Pdfium> = OnceLock::new();

fn locate_worker() -> Option<std::path::PathBuf> {
    if let Ok(p) = std::env::var("PDF_WORKER_PATH") {
        let path = std::path::PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }
    let exe = std::env::current_exe().ok()?;
    let mut dir = exe.parent()?.to_path_buf();
    for _ in 0..3 {
        let candidate = dir.join("pdf_worker");
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = dir.parent()?.to_path_buf();
    }
    None
}

pub fn init_pdfium() -> &'static Pdfium {
    PDFIUM_BINDINGS.get_or_init(|| {
        Pdfium::new(
            Pdfium::bind_to_library(
                Pdfium::pdfium_platform_library_name_at_path("./")
            ).expect("Failed to initialize PDFium bindings")
        )
    })
}

pub fn infer_pdf_semantics(pdf_bytes: &[u8]) -> Result<Vec<Vec<ContentBlock>>, PdfiumError> {
    let pdfium = init_pdfium();
    let document = pdfium.load_pdf_from_byte_slice(pdf_bytes, None)?;
    let page_count = document.pages().len() as usize;
    drop(document);

    if page_count == 0 {
        return Ok(Vec::new());
    }

    let n_workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(page_count);

    let worker_path = locate_worker().expect("pdf_worker binary not found");

    let mut children = Vec::with_capacity(n_workers);
    for w in 0..n_workers {
        let mut child = Command::new(&worker_path)
            .arg(w.to_string())
            .arg(n_workers.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn pdf_worker");

        let mut stdin = child.stdin.take().expect("worker stdin");
        let bytes = pdf_bytes.to_vec();
        let writer = std::thread::spawn(move || {
            let _ = stdin.write_all(&bytes);
        });

        children.push((child, writer));
    }

    let mut all_pages: Vec<Option<Vec<ContentBlock>>> =
        (0..page_count).map(|_| None).collect();

    for (child, writer) in children {
        let _ = writer.join();
        let output = child.wait_with_output().expect("wait pdf_worker");
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("pdf_worker failed ({:?}): {}", output.status, stderr);
        }
        let chunk: Vec<(usize, Vec<ContentBlock>)> =
            serde_json::from_slice(&output.stdout).expect("worker output");
        for (idx, blocks) in chunk {
            if idx < page_count {
                all_pages[idx] = Some(blocks);
            }
        }
    }

    Ok(all_pages
        .into_iter()
        .map(|opt| opt.unwrap_or_default())
        .collect())
}

pub fn extract_pdf_text_with_formatting() -> Result<(), PdfiumError> {
    let bindings = Pdfium::bind_to_library(
        Pdfium::pdfium_platform_library_name_at_path("./")
    )?;

    let pdfium = Pdfium::new(bindings);
    let document = pdfium.load_pdf_from_file("test/qemu_long_pdf.pdf", None)?;


    for (page_index, page) in document.pages().iter().enumerate() {
        println!("\n===== Page {} =====\n", page_index);
        if page_index > 0 {
            break;
        }

        println!("\n===== Width {:?} =====\n", page.width());

        let mut potential_underlines: Vec<(f32, f32, f32, f32)> = Vec::new();
        for object in page.objects().iter() {
            if let Some(path_obj) = object.as_path_object() {
                if let Ok(bounds) = path_obj.bounds() {
                    let top = bounds.top().value;
                    let bottom = bounds.bottom().value;
                    let left = bounds.left().value;
                    let right = bounds.right().value;

                    let height = (top - bottom).abs();
                    let width = (right - left).abs();

                    if height < 3.0 && width > height * 2.0 {
                        potential_underlines.push((left, top, right, bottom));
                    }
                }
            }
        }

        for object in page.objects().iter() {
            if let Some(text_obj) = object.as_text_object() {
                let text = text_obj.text();
                let font = text_obj.font();
                let font_name = font.name();

                println!("Text: {:?}", text);
                println!("  Font name: {:?}", font_name);

                let is_bold = font_name.to_lowercase().contains("bold") || font_name.to_lowercase().contains("heavy");
                let is_italic = font_name.to_lowercase().contains("italic") || font_name.to_lowercase().contains("oblique");

                println!("  Is bold (from name): {}", is_bold);
                println!("  Is italic (from name): {}", is_italic);

                let matrix = text_obj.matrix()?;
                let scale_y = matrix.d();

                let unscaled = text_obj.unscaled_font_size();
                let actual_size = unscaled.value * scale_y.abs() * 0.75;

                println!("  Actual rendered size: {:.2} points, Unscaled size {:.2} points", actual_size.ceil(), unscaled.value);

                if let Ok(text_bounds) = text_obj.bounds() {
                    println!("  Position - Left: {}, Top: {}, Right: {}, Bottom: {}, Width: {}",
                    text_bounds.left(), text_bounds.top(), text_bounds.right(), text_bounds.bottom(), text_bounds.width());
                    let center_right_left_space = (page.width().value - text_bounds.width().value) / 2.0;
                    let left_space_difference = center_right_left_space - text_bounds.left().value;
                    // TODO: I will come back later for right alignment, for now the left and space
                    // let right_space_difference = center_right_left_space - text_bounds.right().value;
                    if left_space_difference.abs() <= 3.0 {
                        println!("  Text Align: Center");
                    } else {
                        println!("  Text Align: Left");
                    }

                    let is_underlined = potential_underlines.iter().any(|(line_left, line_top, line_right, _line_bottom)| {
                        let vertical_gap = text_bounds.bottom().value - line_top;
                        let horizontally_overlaps =
                            *line_left <= text_bounds.right().value &&
                            *line_right >= text_bounds.left().value;
                        vertical_gap >= -2.0 && vertical_gap <= 5.0 && horizontally_overlaps
                    });

                    println!("  Is underlined: {}", is_underlined);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    pub fn can_extract_pdf_text_with_formatting(){
        todo!();
    }

}
pub fn extract_correct_text () {

}