use base64::{Engine as _, engine::general_purpose};
use image::ImageFormat;
use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};
use std::{fmt, io::Cursor};

#[derive(Debug, Clone, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockKind {
    PageNumber,
    Heading1,
    Heading2,
    Heading3,
    Heading4,
    Heading5,
    Heading6,
    HeadingCandidate,
    Epigraph,
    Attribution,
    Paragraph,
    ListItem,
    SubListItem,
    OrderedListItem,
    TableOfContentsHeading,
}

impl fmt::Display for BlockKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockKind::PageNumber => write!(f, "PageNumber"),
            BlockKind::Heading1 => write!(f, "Heading1"),
            BlockKind::Heading2 => write!(f, "Heading2"),
            BlockKind::Heading3 => write!(f, "Heading3"),
            BlockKind::Heading4 => write!(f, "Heading4"),
            BlockKind::Heading5 => write!(f, "Heading5"),
            BlockKind::Heading6 => write!(f, "Heading6"),
            BlockKind::HeadingCandidate => write!(f, "HeadingCandidate"),
            BlockKind::Epigraph => write!(f, "Epigraph"),
            BlockKind::Attribution => write!(f, "Attribution"),
            BlockKind::Paragraph => write!(f, "Paragraph"),
            BlockKind::ListItem => write!(f, "ListItem"),
            BlockKind::SubListItem => write!(f, "SubListItem"),
            BlockKind::OrderedListItem => write!(f, "OrderedListItem"),
            BlockKind::TableOfContentsHeading => write!(f, "TOCHeading"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextFragment {
    pub text: String,
    pub font_name: String,
    // TODO: Remove this field later, just want to confirm font size is being extracted correctly
    pub unscaled: f32,
    pub font_size: f32,
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_underlined: bool,
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone)]
pub struct TextLine {
    pub fragments: Vec<TextFragment>,
    pub top: f32,
    pub bottom: f32,
    pub alignment: TextAlign,
}

impl TextLine {
    pub fn merged_text(&self) -> String {
        let mut result = String::new();
        let mut prev_frag: Option<&TextFragment> = None;

        for frag in self.fragments.iter() {
            if frag.text.is_empty() {
                continue;
            }

            if let Some(prev) = prev_frag {
                let gap = frag.left - prev.right;
                let threshold = prev.font_size.max(frag.font_size) * 0.3;
                let needs_space =
                    gap > threshold && !result.ends_with(' ') && !frag.text.starts_with(' ');
                if needs_space {
                    result.push(' ');
                }
            }

            result.push_str(&frag.text);
            prev_frag = Some(frag);
        }
        result.trim().to_string()
    }

    pub fn font_size(&self) -> f32 {
        self.fragments
            .iter()
            .map(|f| f.font_size)
            .fold(0.0_f32, f32::max)
    }

    pub fn is_bold(&self) -> bool {
        let (bold_chars, total_chars) = self.fragments.iter().fold((0, 0), |(b, t), f| {
            let len = f.text.trim().len();
            (b + if f.is_bold { len } else { 0 }, t + len)
        });
        total_chars > 0 && bold_chars * 2 >= total_chars
    }

    pub fn is_italic(&self) -> bool {
        let (italic_chars, total_chars) = self.fragments.iter().fold((0, 0), |(b, t), f| {
            let len = f.text.trim().len();
            (b + if f.is_italic { len } else { 0 }, t + len)
        });
        total_chars > 0 && italic_chars * 2 >= total_chars
    }

    pub fn is_underlined(&self) -> bool {
        self.fragments.iter().any(|f| f.is_underlined)
    }

    pub fn alignment(&self) -> TextAlign {
        self.alignment.clone()
    }

    pub fn left(&self) -> f32 {
        self.fragments
            .iter()
            .map(|f| f.left)
            .fold(f32::MAX, f32::min)
    }

    pub fn starts_with_bullet(&self) -> bool {
        self.fragments
            .iter()
            .find(|f| !f.text.trim().is_empty())
            .map(|f| f.font_name.contains("Symbol") || f.text.trim().starts_with('•'))
            .unwrap_or(false)
    }

    pub fn starts_with_sub_bullet(&self) -> bool {
        self.fragments
            .iter()
            .find(|f| !f.text.trim().is_empty())
            .map(|f| f.font_name.contains("Courier") && f.text.trim() == "o")
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCell {
    pub text: String,
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_underlined: bool,
    pub col_span: usize,
    pub row_span: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub rows: Vec<TableRow>,
    pub col_count: usize,
    pub y_position: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub data_uri: String,
    pub width_pt: f32,
    pub height_pt: f32,
    pub y_position: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text {
    pub kind: BlockKind,
    pub text: String,
    pub font_size: f32,
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_underlined: bool,
    pub y_position: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentBlock {
    Text(Text),
    Table(Table),
    Image(Image),
}

impl fmt::Display for ContentBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            ContentBlock::Text(text) => write!(f, "[{}] {}", text.kind, text.text),
            ContentBlock::Table(table) => {
                write!(f, "[Table] {}x{}", table.rows.len(), table.col_count)
            }
            ContentBlock::Image(img) => {
                write!(f, "[Image] {:.0}x{:.0}pt", img.width_pt, img.height_pt)
            }
        }
    }
}

impl ContentBlock {
    pub fn y_position(&self) -> f32 {
        match self {
            ContentBlock::Text(t) => t.y_position,
            ContentBlock::Table(t) => t.y_position,
            ContentBlock::Image(i) => i.y_position,
        }
    }
}

pub fn extract_fragments(page: &PdfPage) -> (Vec<TextFragment>, Vec<(f32, f32, f32, f32)>) {
    let mut underlines: Vec<(f32, f32, f32, f32)> = Vec::with_capacity(8);
    let mut structural_paths: Vec<(f32, f32, f32, f32)> = Vec::new();
    let mut fragments: Vec<TextFragment> = Vec::new();

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

                if (height < 3.0 && width > 20.0)
                    || (width < 3.0 && height > 20.0)
                    || (width > 15.0 && height > 10.0)
                {
                    structural_paths.push((left, top, right, bottom));
                }
            }
        } else if let Some(text_obj) = object.as_text_object() {
            let text = text_obj.text();
            if text.trim().is_empty() {
                continue;
            }

            let bounds = match text_obj.bounds() {
                Ok(b) => b,
                Err(_) => continue,
            };
            let left = bounds.left().value;
            let top = bounds.top().value;
            let right = bounds.right().value;
            let bottom = bounds.bottom().value;

            let font_name = text_obj.font().name();
            let lower_name = font_name.to_lowercase();
            let is_bold = lower_name.contains("bold") || lower_name.contains("heavy");
            let is_italic = lower_name.contains("italic") || lower_name.contains("oblique");

            let unscaled = text_obj.unscaled_font_size().value;
            let font_size = if let Ok(matrix) = text_obj.matrix() {
                (unscaled * matrix.d().abs()).ceil()
            } else {
                unscaled
            };

            fragments.push(TextFragment {
                text,
                font_name,
                // TODO: Remove this field later, just want to confirm font size is being extracted correctly
                unscaled,
                font_size,
                is_bold,
                is_italic,
                is_underlined: false,
                left,
                top,
                right,
                bottom,
            });
        }
    }

    if !underlines.is_empty() {
        for frag in fragments.iter_mut() {
            frag.is_underlined = underlines.iter().any(|(ul, ut, ur, _ub)| {
                let vertical_gap = frag.bottom - ut;
                let horiz_overlap = *ul <= frag.right && *ur >= frag.left;
                vertical_gap >= -2.0 && vertical_gap <= 5.0 && horiz_overlap
            });
        }
    }

    (fragments, structural_paths)
}

fn can_merge(prev: &TextFragment, next: &TextFragment) -> bool {
    prev.font_name == next.font_name
        && (prev.font_size - next.font_size).abs() < 0.5
        && prev.is_bold == next.is_bold
        && prev.is_italic == next.is_italic
        && prev.is_underlined == next.is_underlined
}

// Subsetted fonts with per-glyph TJ positioning emit one text object per glyph;
// this folds same-styled neighbours back into word-level fragments.
fn merge_line_fragments(fragments: Vec<TextFragment>) -> Vec<TextFragment> {
    let mut out: Vec<TextFragment> = Vec::with_capacity(fragments.len());
    for frag in fragments {
        let merged = match out.last_mut() {
            Some(prev) if can_merge(prev, &frag) => {
                let gap = frag.left - prev.right;
                let space_threshold = prev.font_size.max(frag.font_size) * 0.3;
                if gap > space_threshold && !prev.text.ends_with(' ') && !frag.text.starts_with(' ')
                {
                    prev.text.push(' ');
                }
                prev.text.push_str(&frag.text);
                prev.right = frag.right;
                if frag.top > prev.top {
                    prev.top = frag.top;
                }
                if frag.bottom < prev.bottom {
                    prev.bottom = frag.bottom;
                }
                true
            }
            _ => false,
        };
        if !merged {
            out.push(frag);
        }
    }
    out
}

// Reject Center when an internal gap reveals the line is actually multi-column.
fn line_alignment(fragments: &[TextFragment], page_width: f32) -> TextAlign {
    if fragments.is_empty() {
        return TextAlign::Left;
    }
    let leftmost = fragments.iter().map(|f| f.left).fold(f32::MAX, f32::min);
    let rightmost = fragments.iter().map(|f| f.right).fold(f32::MIN, f32::max);
    let line_width = rightmost - leftmost;
    let center_margin = (page_width - line_width) / 2.0;
    let left_diff = (center_margin - leftmost).abs();

    let max_internal_gap = fragments
        .windows(2)
        .map(|pair| pair[1].left - pair[0].right)
        .fold(0.0_f32, f32::max);
    let has_column_gap = max_internal_gap > page_width * 0.1;

    if left_diff <= 3.0 && !has_column_gap {
        TextAlign::Center
    } else {
        TextAlign::Left
    }
}

struct LineCluster {
    fragments: Vec<TextFragment>,
    max_font_size: f32,
    top: f32,
    bottom: f32,
}

impl LineCluster {
    fn new(frag: TextFragment) -> Self {
        Self {
            max_font_size: frag.font_size,
            top: frag.top,
            bottom: frag.bottom,
            fragments: vec![frag],
        }
    }

    fn accepts(&self, frag: &TextFragment) -> bool {
        let overlap = self.top.min(frag.top) - self.bottom.max(frag.bottom);
        let min_fs = self.max_font_size.min(frag.font_size).max(0.1);
        let max_fs = self.max_font_size.max(frag.font_size);
        overlap > 0.5 && max_fs / min_fs < 1.5
    }

    fn absorb(&mut self, frag: TextFragment) {
        if frag.font_size > self.max_font_size {
            self.max_font_size = frag.font_size;
        }
        if frag.top > self.top {
            self.top = frag.top;
        }
        if frag.bottom < self.bottom {
            self.bottom = frag.bottom;
        }
        self.fragments.push(frag);
    }
}

// Assigns each fragment to any matching cluster, not just the most recent —
// a single-active-line walk drops fragments when sibling lines interleave in
// sort order.
pub fn group_into_lines(mut fragments: Vec<TextFragment>, page_width: f32) -> Vec<TextLine> {
    if fragments.is_empty() {
        return Vec::new();
    }

    fragments.sort_by(|a, b| {
        b.top
            .partial_cmp(&a.top)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    

    let mut clusters: Vec<LineCluster> = Vec::new();

    // eprintln!("These are the remaining fragments:");
    
    for frag in fragments {
        // eprintln!("{:?} \n", frag);
        match clusters.iter().position(|c| c.accepts(&frag)) {
            Some(idx) => clusters[idx].absorb(frag),
            None => clusters.push(LineCluster::new(frag)),
        }
    }

    clusters.sort_by(|a, b| {
        b.top
            .partial_cmp(&a.top)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    clusters
        .into_iter()
        .map(|c| {
            let mut frags = c.fragments;
            frags.sort_by(|a, b| {
                a.left
                    .partial_cmp(&b.left)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let merged = merge_line_fragments(frags);
            let alignment = line_alignment(&merged, page_width);
            TextLine {
                fragments: merged,
                top: c.top,
                bottom: c.bottom,
                alignment,
            }
        })
        .collect()
}

fn ordered_marker_len(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
    if digits == 0 || digits > 3 {
        return None;
    }
    // PDFs may emit whitespace between the number and its delimiter.
    let mut i = digits;
    while bytes.get(i) == Some(&b' ') {
        i += 1;
    }
    let delim = *bytes.get(i)?;
    if delim != b'.' && delim != b')' {
        return None;
    }
    if text[i + 1..].trim_start().is_empty() {
        return None;
    }
    Some(i + 1)
}

fn looks_complete(text: &str) -> bool {
    matches!(
        text.trim().chars().last(),
        Some('.' | '!' | '?' | ':' | ';')
    )
}

fn line_right(line: &TextLine) -> f32 {
    line.fragments
        .iter()
        .map(|f| f.right)
        .fold(f32::MIN, f32::max)
}

fn normalize_ordered_marker(text: &str) -> String {
    let Some(len) = ordered_marker_len(text) else {
        return text.to_string();
    };
    let digits = text.bytes().take_while(|b| b.is_ascii_digit()).count();
    let delim = text.as_bytes()[len - 1] as char;
    format!("{}{} {}", &text[..digits], delim, text[len..].trim_start())
}

fn classify_line(line: &TextLine, is_in_toc: bool) -> BlockKind {
    let text = line.merged_text();
    let font_size = line.font_size();
    let alignment = line.alignment();
    let bold = line.is_bold();
    let italic = line.is_italic();
    let underlined = line.is_underlined();

    if text.trim().parse::<u32>().is_ok() && alignment == TextAlign::Center && font_size <= 11.0 {
        return BlockKind::PageNumber;
    }

    if line.starts_with_bullet() {
        return BlockKind::ListItem;
    }

    if line.starts_with_sub_bullet() {
        return BlockKind::SubListItem;
    }

    if alignment == TextAlign::Left && ordered_marker_len(&text).is_some() {
        return BlockKind::OrderedListItem;
    }

    if bold && underlined && text.contains("Table of Contents") {
        return BlockKind::TableOfContentsHeading;
    }

    if alignment == TextAlign::Center && underlined && text.len() < 120 {
        return BlockKind::HeadingCandidate;
    }

    if alignment == TextAlign::Center && italic {
        return BlockKind::Epigraph;
    }

    if alignment == TextAlign::Center && !italic && !bold && text.len() < 40 {
        return BlockKind::Attribution;
    }

    if alignment == TextAlign::Center && bold && text.len() < 120 {
        return BlockKind::HeadingCandidate;
    }

    if alignment == TextAlign::Center && !italic && !bold && font_size <= 11.0 && text.len() < 60 {
        return BlockKind::HeadingCandidate;
    }

    if is_in_toc && bold {
        return BlockKind::ListItem;
    }

    if bold && alignment == TextAlign::Left && text.len() < 120 {
        return BlockKind::HeadingCandidate;
    }

    BlockKind::Paragraph
}

pub fn merge_into_blocks(lines: Vec<TextLine>) -> Vec<ContentBlock> {
    if lines.is_empty() {
        return Vec::new();
    }

    let content_right = lines
        .iter()
        .flat_map(|l| l.fragments.iter().map(|f| f.right))
        .fold(f32::MIN, f32::max);

    let mut blocks: Vec<ContentBlock> = Vec::new();
    let mut in_toc = false;
    let mut prev_line_bottom: Option<f32> = None;
    let mut prev_line_right: Option<f32> = None;

    for line in &lines {
        let text = line.merged_text();
        if text.is_empty() {
            continue;
        }

        if text.contains("Table of Contents") {
            in_toc = true;
        }

        let kind = classify_line(line, in_toc);

        let clean_text = if kind == BlockKind::ListItem && line.starts_with_bullet() {
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
        } else if kind == BlockKind::OrderedListItem {
            normalize_ordered_marker(&text)
        } else {
            text
        };

        if clean_text.is_empty() {
            continue;
        }

        // PDF Y grows upward, so prev.bottom sits above current.top; a gap
        // exceeding ~1.3× the line's font size marks a paragraph boundary.
        let has_paragraph_break = if let Some(prev_bottom) = prev_line_bottom {
            prev_bottom - line.top > line.font_size() * 1.3
        } else {
            false
        };

        let should_merge = if let Some(ContentBlock::Text(prev_text)) = blocks.last() {
            match (&prev_text.kind, &kind) {
                (BlockKind::Paragraph, BlockKind::Paragraph)
                | (BlockKind::Epigraph, BlockKind::Epigraph)
                | (
                    BlockKind::OrderedListItem | BlockKind::ListItem | BlockKind::SubListItem,
                    BlockKind::Paragraph,
                ) => !has_paragraph_break,
                (BlockKind::HeadingCandidate, BlockKind::HeadingCandidate) => {
                    let same_style = (prev_text.font_size - line.font_size()).abs() < 0.5
                        && prev_text.is_bold == line.is_bold()
                        && prev_text.is_italic == line.is_italic();
                    let filled_width = prev_line_right
                        .map(|r| content_right - r < line.font_size() * 2.0)
                        .unwrap_or(false);
                    !has_paragraph_break
                        && same_style
                        && filled_width
                        && !looks_complete(&prev_text.text)
                }
                _ => false,
            }
        } else {
            false
        };

        if should_merge {
            if let Some(ContentBlock::Text(prev_text)) = blocks.last_mut() {
                prev_text.text.push(' ');
                prev_text.text.push_str(&clean_text);
            }
        } else {
            blocks.push(ContentBlock::Text(Text {
                kind,
                text: clean_text,
                font_size: line.font_size(),
                is_bold: line.is_bold(),
                is_italic: line.is_italic(),
                is_underlined: line.is_underlined(),
                y_position: line.top,
            }));
        }

        prev_line_bottom = Some(line.bottom);
        prev_line_right = Some(line_right(line));
    }

    // eprintln!("These are the final blocks: {:?}", blocks);

    blocks
}

pub fn reconstruct_page(page: &PdfPage) -> Vec<ContentBlock> {
    let page_width = page.width().value;
    let (fragments, structural_paths) = extract_fragments(page);

    // Subsetted fonts emit one text object per glyph; merge into runs before
    // table detection so each line isn't read as a row of single-glyph columns.
    let merged_fragments: Vec<TextFragment> = group_into_lines(fragments, page_width)
        .into_iter()
        .flat_map(|line| line.fragments)
        .collect();

    let (remaining_fragments, table_blocks) =
        super::table::detect_and_extract_tables(merged_fragments, &structural_paths, page_width);
    let lines = group_into_lines(remaining_fragments, page_width);
    let mut all_blocks = merge_into_blocks(lines);
    all_blocks.extend(table_blocks);
    all_blocks.extend(extract_images(page));
    all_blocks.sort_by(|a, b| {
        b.y_position()
            .partial_cmp(&a.y_position())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all_blocks
}

fn extract_images(page: &PdfPage) -> Vec<ContentBlock> {
    let mut image_blocks: Vec<ContentBlock> = Vec::new();

    for object in page.objects().iter() {
        let Some(image_obj) = object.as_image_object() else {
            continue;
        };

        let bounds = match image_obj.bounds() {
            Ok(b) => b,
            Err(_) => continue,
        };

        let top = bounds.top().value;
        let bottom = bounds.bottom().value;
        let left = bounds.left().value;
        let right = bounds.right().value;
        let width_pt = (right - left).abs();
        let height_pt = (top - bottom).abs();

        if width_pt < 50.0 || height_pt < 50.0 {
            continue;
        }

        let bitmap = match image_obj.get_raw_bitmap() {
            Ok(b) => b,
            Err(_) => continue,
        };

        let dynamic_image = match bitmap.as_image() {
            Ok(img) => img,
            Err(_) => continue,
        };

        if dynamic_image.width() < 50 || dynamic_image.height() < 50 {
            continue;
        }

        let dynamic_image = if dynamic_image.width() > 1200 || dynamic_image.height() > 1200 {
            dynamic_image.thumbnail(1200, 1200)
        } else {
            dynamic_image
        };

        let rgb = dynamic_image.to_rgb8();
        let mut buf = Cursor::new(Vec::<u8>::new());
        if image::DynamicImage::ImageRgb8(rgb)
            .write_to(&mut buf, ImageFormat::Jpeg)
            .is_err()
        {
            continue;
        }

        let data_uri = format!(
            "data:image/jpeg;base64,{}",
            general_purpose::STANDARD.encode(buf.into_inner())
        );

        image_blocks.push(ContentBlock::Image(Image {
            data_uri,
            width_pt,
            height_pt,
            y_position: top,
        }));
    }

    image_blocks
}

pub fn assign_heading_levels(pages: &mut [Vec<ContentBlock>]) {
    let mut body_sizes: Vec<f32> = pages
        .iter()
        .flatten()
        .filter_map(|b| match b {
            ContentBlock::Text(t) if t.kind == BlockKind::Paragraph => Some(t.font_size),
            _ => None,
        })
        .collect();

    let body_size = if body_sizes.is_empty() {
        10.0
    } else {
        body_sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        body_sizes[body_sizes.len() / 2]
    };

    let mut sizes: Vec<f32> = pages
        .iter()
        .flatten()
        .filter_map(|b| match b {
            ContentBlock::Text(t) if t.kind == BlockKind::HeadingCandidate => Some(t.font_size),
            _ => None,
        })
        .collect();

    if sizes.is_empty() {
        return;
    }

    sizes.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    let mut distinct_levels: Vec<f32> = Vec::new();
    for size in &sizes {
        if distinct_levels.iter().all(|l| (l - size).abs() >= 0.5) {
            distinct_levels.push(*size);
        }
    }

    let ratio = distinct_levels[0] / body_size;
    let starting_offset: usize = if ratio >= 1.4 {
        0
    } else if ratio >= 1.25 {
        1
    } else if ratio >= 1.15 {
        2
    } else if ratio >= 1.05 {
        3
    } else if ratio >= 0.95 {
        4
    } else {
        5
    };

    let level_for = |size: f32| -> BlockKind {
        let idx = distinct_levels
            .iter()
            .position(|l| (l - size).abs() < 0.5)
            .unwrap_or(distinct_levels.len().saturating_sub(1));
        match (idx + starting_offset).min(5) {
            0 => BlockKind::Heading1,
            1 => BlockKind::Heading2,
            2 => BlockKind::Heading3,
            3 => BlockKind::Heading4,
            4 => BlockKind::Heading5,
            _ => BlockKind::Heading6,
        }
    };

    for page in pages.iter_mut() {
        for block in page.iter_mut() {
            if let ContentBlock::Text(t) = block {
                if t.kind == BlockKind::HeadingCandidate {
                    t.kind = level_for(t.font_size);
                }
            }
        }
    }
}
