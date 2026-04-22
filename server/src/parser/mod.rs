use pdfium_render::prelude::*;

pub mod reconstruct;

use reconstruct::reconstruct_page;

pub fn extract_structured_text() -> Result<(), PdfiumError> {
    let bindings = Pdfium::bind_to_library(
        Pdfium::pdfium_platform_library_name_at_path("./")
    )?;

    let pdfium = Pdfium::new(bindings);
    let document = pdfium.load_pdf_from_file("test/short_pdf.pdf", None)?;

    for (page_index, page) in document.pages().iter().enumerate() {
        println!("\n===== Page {} =====\n", page_index + 1);

        let blocks = reconstruct_page(&page);

        for block in &blocks {
            println!("{}", block);
        }
    }

    Ok(())
}

pub fn extract_pdf_text_with_formatting() -> Result<(), PdfiumError> {
    let bindings = Pdfium::bind_to_library(
        Pdfium::pdfium_platform_library_name_at_path("./")
    )?;

    let pdfium = Pdfium::new(bindings);
    let document = pdfium.load_pdf_from_file("test/short_pdf.pdf", None)?;


    for (page_index, page) in document.pages().iter().enumerate() {
        println!("\n===== Page {} =====\n", page_index);

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