use pdfium_render::prelude::*;
use serde::{Serialize, Deserialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockKind {
    PageNumber,
    Title,
    Subtitle,
    Epigraph,
    Attribution,
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
}

#[derive(Debug, Clone)]
pub struct TextLine {
    pub fragments: Vec<TextFragment>,
    pub top: f32,
    pub bottom: f32,
    pub alignment: TextAlign,
}

impl TextLine {
    /// Merge all fragment text into one string, inserting spaces between
    /// fragments that aren't already separated.
    pub fn merged_text(&self) -> String {
        let mut result = String::new();
        let mut prev_frag: Option<&TextFragment> = None;

        for frag in self.fragments.iter() {
            if frag.text.is_empty() {
                continue;
            }

            if let Some(prev) = prev_frag {
                // Calculate horizontal gap between current fragment's left and previous fragment's right
                let gap = frag.left - prev.right;

                let threshold = prev.font_size.max(frag.font_size) * 0.3;

                // Add space if there is a wide enough gap AND I don't already have one
                let needs_space = gap > threshold && !result.ends_with(' ') && !frag.text.starts_with(' ');
                if needs_space {
                    result.push(' ');
                }
            }
            
            result.push_str(&frag.text);
            prev_frag = Some(frag);
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

    /// Alignment of this line (computed from the line's combined bounds
    /// against page width, not from any single fragment).
    pub fn alignment(&self) -> TextAlign {
        self.alignment.clone()
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
                });
            }
        }
    }

    (fragments, underlines)
}


/// Two fragments can fold into one if every style attribute matches.
/// Caller is responsible for ensuring they're already on the same line.
fn can_merge(prev: &TextFragment, next: &TextFragment) -> bool {
    prev.font_name == next.font_name
        && (prev.font_size - next.font_size).abs() < 0.5
        && prev.is_bold == next.is_bold
        && prev.is_italic == next.is_italic
        && prev.is_underlined == next.is_underlined
}


/// Collapse adjacent fragments on a line that share the same style into
/// single fragments. PDFs that use subsetted fonts with per-glyph TJ
/// positioning emit one text object per glyph; this folds those back into
/// word/run-level fragments. Inserts a space when the horizontal gap is
/// large enough to represent a real word boundary.
///
/// Expects fragments already sorted left-to-right.
fn merge_line_fragments(fragments: Vec<TextFragment>) -> Vec<TextFragment> {
    let mut out: Vec<TextFragment> = Vec::with_capacity(fragments.len());
    for frag in fragments {
        let merged = match out.last_mut() {
            Some(prev) if can_merge(prev, &frag) => {
                let gap = frag.left - prev.right;
                let space_threshold = prev.font_size.max(frag.font_size) * 0.3;
                if gap > space_threshold
                    && !prev.text.ends_with(' ')
                    && !frag.text.starts_with(' ')
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


/// Compute alignment for a finished line. Reject Center when there's a
/// big internal gap — a contiguous span with symmetric outer margins is
/// centered; a span with an empty middle is multi-column.
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


/// In-progress line cluster used during fragment grouping.
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

    /// Same-line test: vertical bbox overlap, with a font-size ratio cap
    /// to keep different-size text in separate lines when their bands
    /// happen to overlap.
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

/// Group fragments into lines, then merge adjacent same-style fragments
/// within each line.
///
/// Each fragment is assigned to any matching cluster in the active set,
/// not just the most recent. A single-active-line walk fails when
/// fragments from different lines interleave in sort order — the loop
/// would keep closing and reopening lines.
pub fn group_into_lines(mut fragments: Vec<TextFragment>, page_width: f32) -> Vec<TextLine> {
    if fragments.is_empty() {
        return Vec::new();
    }

    fragments.sort_by(|a, b| b.top.partial_cmp(&a.top).unwrap_or(std::cmp::Ordering::Equal));

    let mut clusters: Vec<LineCluster> = Vec::new();

    for frag in fragments {
        match clusters.iter().position(|c| c.accepts(&frag)) {
            Some(idx) => clusters[idx].absorb(frag),
            None => clusters.push(LineCluster::new(frag)),
        }
    }

    clusters.sort_by(|a, b| b.top.partial_cmp(&a.top).unwrap_or(std::cmp::Ordering::Equal));

    clusters
        .into_iter()
        .map(|c| {
            let mut frags = c.fragments;
            frags.sort_by(|a, b| {
                a.left.partial_cmp(&b.left).unwrap_or(std::cmp::Ordering::Equal)
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
/// Uses vertical gap detection to identify paragraph boundaries: if the gap
/// between two consecutive lines exceeds normal line spacing (1.3× font size),
/// they belong to separate paragraphs.
pub fn merge_into_blocks(lines: Vec<TextLine>) -> Vec<ContentBlock> {
    if lines.is_empty() {
        return Vec::new();
    }

    let mut blocks: Vec<ContentBlock> = Vec::new();
    let mut in_toc = false;
    // Track the bottom coordinate of the previous line so I can measure
    // vertical gaps between consecutive lines.
    let mut prev_line_bottom: Option<f32> = None;

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

        // Measure the vertical gap between this line and the previous one.
        // In PDF coordinates, Y increases upward, so the previous line's
        // bottom is ABOVE the current line's top.
        // gap = prev_bottom - current_top  (positive = normal spacing)
        // Within a paragraph, gap ≈ font_size × 1.2 (normal leading).
        // Between paragraphs, gap is noticeably larger.
        let has_paragraph_break = if let Some(prev_bottom) = prev_line_bottom {
            let gap = prev_bottom - line.top;
            let font_size = line.font_size();
            // A gap larger than 1.3× the font size indicates a paragraph break.
            // Normal line spacing is ~1.2× font size, so 1.3× gives comfortable margin.
            gap > font_size * 1.3
        } else {
            false
        };

        // Determine if the current line should merge with the previous block
        let should_merge = if let Some(prev) = blocks.last() {
            match (&prev.kind, &kind) {
                // Merge consecutive paragraph lines only if there's no paragraph break
                (BlockKind::Paragraph, BlockKind::Paragraph) => !has_paragraph_break,
                // Merge consecutive epigraph lines only if there's no paragraph break
                (BlockKind::Epigraph, BlockKind::Epigraph) => !has_paragraph_break,
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

        // Update the previous line's bottom for the next iteration
        prev_line_bottom = Some(line.bottom);
    }

    blocks
}


/// Reconstruct a single page into semantic content blocks.
pub fn reconstruct_page(page: &PdfPage) -> Vec<ContentBlock> {
    let (fragments, _underlines) = extract_fragments(page);
    let lines = group_into_lines(fragments, page.width().value);
    merge_into_blocks(lines)
}
