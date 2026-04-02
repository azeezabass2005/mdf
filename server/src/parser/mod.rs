use pdfium_render::prelude::*;

pub fn extract_pdf_text() -> Result<(), PdfiumError> {

    let bindings = Pdfium::bind_to_library(
        Pdfium::pdfium_platform_library_name_at_path("./")
    )?;


    Pdfium::new(bindings)
        .load_pdf_from_file("test/our_pdf.pdf", None)?
        .pages()
        .iter()
        .enumerate()
        .for_each(|(_index, page)| {

            println!("{}", page.text().unwrap().all());

        });
        Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
 
    #[test]
    pub fn can_extract_pdf_text(){
        let extracted_text = extract_pdf_text();
    }

}
pub fn extract_correct_text () {

}