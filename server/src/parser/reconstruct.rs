use pdfium_render::prelude::*;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockKind {
    PageNumber,
    Title,
    Subtitle,
    Epigraph,
    Attribution,
    // TODO: I will work on splitting this into ParagraphStart and MidParagraph
    // The current implementation is not correctly tracking if it's a new paragraph,
    // The should_merge function is combining different paragraphs and not distinguishing between fragments piece marked as Paragraph
    Paragraph,
    Heading,
    ListItem,
    SubListItem,
    TableOfContentsHeading,
}

impl fmt::Display for BlockKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockKind::PageNumber => write!(f, "PageNumber"),
            BlockKind::Title => write!(f, "Title"),
            BlockKind::Subtitle => write!(f, "Subtitle"),
            BlockKind::Epigraph => write!(f, "Epigraph"),
            BlockKind::Attribution => write!(f, "Attribution"),
            BlockKind::Paragraph => write!(f, "Paragraph"),
            BlockKind::Heading => write!(f, "Heading"),
            BlockKind::ListItem => write!(f, "ListItem"),
            BlockKind::SubListItem => write!(f, "SubListItem"),
            BlockKind::TableOfContentsHeading => write!(f, "TOCHeading"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextFragment {
    pub text: String,
    pub font_name: String,
    pub font_size: f32,
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_underlined: bool,
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub alignment: TextAlign,
}

#[derive(Debug, Clone)]
pub struct TextLine {
    pub fragments: Vec<TextFragment>,
    pub top: f32,
    pub bottom: f32,
}

impl TextLine {
    /// Merge all fragment text into one string, inserting spaces between
    /// fragments that aren't already separated.
    pub fn merged_text(&self) -> String {
        let mut result = String::new();
        for (i, frag) in self.fragments.iter().enumerate() {
            if i > 0 {
                // Add space between fragments if the previous doesn't end with
                // a space and the current doesn't start with one.
                let needs_space = !result.ends_with(' ') && !frag.text.starts_with(' ');
                if needs_space {
                    result.push(' ');
                }
            }
            result.push_str(&frag.text);
        }
        result.trim().to_string()
    }

    /// Dominant font size on this line (the max across fragments).
    pub fn font_size(&self) -> f32 {
        self.fragments
            .iter()
            .map(|f| f.font_size)
            .fold(0.0_f32, f32::max)
    }

    /// Whether the majority of non-empty text on this line is bold.
    pub fn is_bold(&self) -> bool {
        let (bold_chars, total_chars) = self.fragments.iter().fold((0, 0), |(b, t), f| {
            let len = f.text.trim().len();
            (b + if f.is_bold { len } else { 0 }, t + len)
        });
        total_chars > 0 && bold_chars * 2 >= total_chars
    }

    /// Whether the majority of non-empty text on this line is italic.
    pub fn is_italic(&self) -> bool {
        let (italic_chars, total_chars) = self.fragments.iter().fold((0, 0), |(b, t), f| {
            let len = f.text.trim().len();
            (b + if f.is_italic { len } else { 0 }, t + len)
        });
        total_chars > 0 && italic_chars * 2 >= total_chars
    }

    /// Whether any fragment on this line is underlined.
    pub fn is_underlined(&self) -> bool {
        self.fragments.iter().any(|f| f.is_underlined)
    }

    /// Dominant alignment (first non-empty fragment).
    pub fn alignment(&self) -> TextAlign {
        self.fragments
            .iter()
            .find(|f| !f.text.trim().is_empty())
            .map(|f| f.alignment.clone())
            .unwrap_or(TextAlign::Left)
    }

    /// Left edge of the leftmost fragment.
    pub fn left(&self) -> f32 {
        self.fragments
            .iter()
            .map(|f| f.left)
            .fold(f32::MAX, f32::min)
    }

    /// Whether this line starts with a bullet character (Symbol font "•").
    pub fn starts_with_bullet(&self) -> bool {
        self.fragments
            .iter()
            .find(|f| !f.text.trim().is_empty())
            .map(|f| f.font_name.contains("Symbol") || f.text.trim().starts_with('•'))
            .unwrap_or(false)
    }

    /// Whether this line starts with a sub-bullet "o" in Courier font.
    pub fn starts_with_sub_bullet(&self) -> bool {
        self.fragments
            .iter()
            .find(|f| !f.text.trim().is_empty())
            .map(|f| f.font_name.contains("Courier") && f.text.trim() == "o")
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub struct ContentBlock {
    pub kind: BlockKind,
    pub text: String,
    pub font_size: f32,
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_underlined: bool,
}

impl fmt::Display for ContentBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.kind, self.text)
    }
}


/// Extract text fragments and underline paths from a single page.
pub fn extract_fragments(
    page: &PdfPage,
) -> (Vec<TextFragment>, Vec<(f32, f32, f32, f32)>) {
    let page_width = page.width().value;

    // Collection of potential underline paths
    let mut underlines: Vec<(f32, f32, f32, f32)> = Vec::new();
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
                    underlines.push((left, top, right, bottom));
                }
            }
        }
    }

    //  Collection of text fragments
    let mut fragments: Vec<TextFragment> = Vec::new();
    for object in page.objects().iter() {
        if let Some(text_obj) = object.as_text_object() {
            let text = text_obj.text();

            // Skip zero-width empty strings
            if text.trim().is_empty() {
                continue;
            }

            let font = text_obj.font();
            let font_name = font.name();
            let lower_name = font_name.to_lowercase();

            let is_bold = lower_name.contains("bold") || lower_name.contains("heavy");
            let is_italic = lower_name.contains("italic") || lower_name.contains("oblique");

            // Compute actual font size
            let font_size = if let Ok(matrix) = text_obj.matrix() {
                let scale_y = matrix.d();
                let unscaled = text_obj.unscaled_font_size().value;
                (unscaled * scale_y.abs() * 0.75).ceil()
            } else {
                text_obj.unscaled_font_size().value
            };

            if let Ok(bounds) = text_obj.bounds() {
                let left = bounds.left().value;
                let top = bounds.top().value;
                let right = bounds.right().value;
                let bottom = bounds.bottom().value;
                let width = bounds.width().value;

                // Determine alignment
                let center_margin = (page_width - width) / 2.0;
                let left_diff = (center_margin - left).abs();
                let alignment = if left_diff <= 3.0 {
                    TextAlign::Center
                } else {
                    TextAlign::Left
                };

                // Check underline
                let is_underlined = underlines.iter().any(|(ul, ut, ur, _ub)| {
                    let vertical_gap = bottom - ut;
                    let horiz_overlap = *ul <= right && *ur >= left;
                    vertical_gap >= -2.0 && vertical_gap <= 5.0 && horiz_overlap
                });

                fragments.push(TextFragment {
                    text,
                    font_name,
                    font_size,
                    is_bold,
                    is_italic,
                    is_underlined,
                    left,
                    top,
                    right,
                    bottom,
                    alignment,
                });
            }
        }
    }

    (fragments, underlines)
}


/// Group fragments into lines based on vertical proximity.
/// Fragments whose vertical ranges overlap within a threshold are on the same line.
pub fn group_into_lines(mut fragments: Vec<TextFragment>) -> Vec<TextLine> {
    if fragments.is_empty() {
        return Vec::new();
    }

    // Sort by descending top (page coordinates: top of page = high value).
    fragments.sort_by(|a, b| b.top.partial_cmp(&a.top).unwrap_or(std::cmp::Ordering::Equal));

    let mut lines: Vec<TextLine> = Vec::new();
    let mut current_fragments: Vec<TextFragment> = vec![fragments[0].clone()];
    let mut current_top = fragments[0].top;
    let mut current_bottom = fragments[0].bottom;

    let vertical_threshold = 3.0; // points

    for frag in fragments.iter().skip(1) {
        // Check if this fragment belongs on the current line by comparing
        // the fragment's vertical midpoint against the current line's range.
        // This is stricter than edge-based overlap and prevents items on
        // adjacent lines from merging when their edges barely touch.
        let frag_mid = (frag.top + frag.bottom) / 2.0;
        let line_mid = (current_top + current_bottom) / 2.0;
        let on_same_line = (frag_mid - line_mid).abs() <= vertical_threshold;

        if on_same_line {
            current_fragments.push(frag.clone());
            // Expand the vertical bounds of the line
            if frag.top > current_top {
                current_top = frag.top;
            }
            if frag.bottom < current_bottom {
                current_bottom = frag.bottom;
            }
        } else {
            // Finish the current line
            current_fragments.sort_by(|a, b| {
                a.left.partial_cmp(&b.left).unwrap_or(std::cmp::Ordering::Equal)
            });
            lines.push(TextLine {
                fragments: current_fragments,
                top: current_top,
                bottom: current_bottom,
            });
            // Start a new line
            current_fragments = vec![frag.clone()];
            current_top = frag.top;
            current_bottom = frag.bottom;
        }
    }

    // Don't forget the last line
    if !current_fragments.is_empty() {
        current_fragments.sort_by(|a, b| {
            a.left.partial_cmp(&b.left).unwrap_or(std::cmp::Ordering::Equal)
        });
        lines.push(TextLine {
            fragments: current_fragments,
            top: current_top,
            bottom: current_bottom,
        });
    }

    lines
}


/// Classify a line into a block kind, based on its style properties.
fn classify_line(line: &TextLine, is_in_toc: bool) -> BlockKind {
    let text = line.merged_text();
    let font_size = line.font_size();
    let alignment = line.alignment();
    let bold = line.is_bold();
    let italic = line.is_italic();
    let underlined = line.is_underlined();

    // Page numbers: small, centered, very short numeric text
    if text.trim().parse::<u32>().is_ok() && alignment == TextAlign::Center && font_size <= 9.0 {
        return BlockKind::PageNumber;
    }

    // Bullet list items
    if line.starts_with_bullet() {
        return BlockKind::ListItem;
    }

    // Sub-bullet list items
    if line.starts_with_sub_bullet() {
        return BlockKind::SubListItem;
    }

    // Title: centered, underlined, larger font
    if alignment == TextAlign::Center && underlined && font_size >= 12.0 {
        return BlockKind::Title;
    }

    // "Table of Contents" heading
    if bold && underlined && text.contains("Table of Contents") {
        return BlockKind::TableOfContentsHeading;
    }

    // Subtitle: centered, smaller than title, not italic
    if alignment == TextAlign::Center && !italic && !bold && font_size <= 9.0 && text.len() < 60 {
        // Likely author/subtitle line
        return BlockKind::Subtitle;
    }

    // Epigraph: centered + italic
    if alignment == TextAlign::Center && italic {
        return BlockKind::Epigraph;
    }

    // Attribution: centered, not italic, not bold, short
    if alignment == TextAlign::Center && !italic && !bold && text.len() < 40 {
        return BlockKind::Attribution;
    }

    // TOC bold entries within the TOC section
    if is_in_toc && bold {
        return BlockKind::ListItem;
    }

    // Heading: bold, left-aligned
    if bold && alignment == TextAlign::Left {
        return BlockKind::Heading;
    }

    // Default: paragraph
    BlockKind::Paragraph
}


/// Merge classified lines into content blocks. Consecutive lines with the
/// same classification and compatible styles are joined into a single block.
pub fn merge_into_blocks(lines: Vec<TextLine>) -> Vec<ContentBlock> {
    if lines.is_empty() {
        return Vec::new();
    }

    let mut blocks: Vec<ContentBlock> = Vec::new();
    let mut in_toc = false;

    for line in &lines {
        let text = line.merged_text();
        if text.is_empty() {
            continue;
        }

        // Track whether it is in the TOC section
        if text.contains("Table of Contents") {
            in_toc = true;
        }

        let kind = classify_line(line, in_toc);

        // For list items with bullet/sub-bullet, strip the bullet prefix from
        // text and use the content after it.
        let clean_text = if kind == BlockKind::ListItem && line.starts_with_bullet() {
            // Get the text from all fragments after the bullet symbol fragment
            let bullet_idx = line
                .fragments
                .iter()
                .position(|f| f.font_name.contains("Symbol") || f.text.trim().starts_with('•'))
                .unwrap_or(0);
            line.fragments[bullet_idx + 1..]
                .iter()
                .map(|f| f.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string()
        } else if kind == BlockKind::SubListItem && line.starts_with_sub_bullet() {
            // Get text from fragments after the "o" marker
            let marker_idx = line
                .fragments
                .iter()
                .position(|f| f.font_name.contains("Courier") && f.text.trim() == "o")
                .unwrap_or(0);
            line.fragments[marker_idx + 1..]
                .iter()
                .map(|f| f.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string()
        } else {
            text
        };

        if clean_text.is_empty() {
            continue;
        }

        // Determine if the current line should merge with the previous block
        let should_merge = if let Some(prev) = blocks.last() {
            match (&prev.kind, &kind) {
                // Merge consecutive paragraph lines
                (BlockKind::Paragraph, BlockKind::Paragraph) => true,
                // Merge consecutive epigraph lines
                (BlockKind::Epigraph, BlockKind::Epigraph) => true,
                _ => false,
            }
        } else {
            false
        };

        if should_merge {
            let prev = blocks.last_mut().unwrap();
            prev.text.push(' ');
            prev.text.push_str(&clean_text);
        } else {
            blocks.push(ContentBlock {
                kind,
                text: clean_text,
                font_size: line.font_size(),
                is_bold: line.is_bold(),
                is_italic: line.is_italic(),
                is_underlined: line.is_underlined(),
            });
        }
    }

    blocks
}


/// Reconstruct a single page into semantic content blocks.
pub fn reconstruct_page(page: &PdfPage) -> Vec<ContentBlock> {
    let (fragments, _underlines) = extract_fragments(page);
    let lines = group_into_lines(fragments);
    merge_into_blocks(lines)
}
