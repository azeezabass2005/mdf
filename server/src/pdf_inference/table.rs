use super::reconstruct::{ContentBlock, Table, TableCell, TableRow, TextFragment};

#[derive(Debug, Clone, Copy)]
struct Rect {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl Rect {
    fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left: left.min(right),
            top: top.max(bottom),
            right: right.max(left),
            bottom: bottom.min(top),
        }
    }

    fn width(self) -> f32 {
        self.right - self.left
    }

    fn height(self) -> f32 {
        self.top - self.bottom
    }

    fn contains_center(self, frag: &TextFragment) -> bool {
        let cx = (frag.left + frag.right) / 2.0;
        let cy = (frag.top + frag.bottom) / 2.0;
        cx >= self.left && cx <= self.right && cy >= self.bottom && cy <= self.top
    }

    fn union(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.max(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.min(other.bottom),
        }
    }

    fn overlaps_or_adjacent(self, other: Self, tolerance: f32) -> bool {
        self.left - tolerance <= other.right
            && self.right + tolerance >= other.left
            && self.bottom - tolerance <= other.top
            && self.top + tolerance >= other.bottom
    }
}

pub fn detect_and_extract_tables(
    fragments: Vec<TextFragment>,
    structural_paths: &[(f32, f32, f32, f32)],
    page_width: f32,
) -> (Vec<TextFragment>, Vec<ContentBlock>) {
    let table_regions = find_table_regions(structural_paths, &fragments, page_width);

    if table_regions.is_empty() {
        return (fragments, Vec::new());
    }

    let mut claimed = vec![false; fragments.len()];
    let mut table_blocks: Vec<ContentBlock> = Vec::new();

    for region in &table_regions {
        let indexed: Vec<(usize, &TextFragment)> = fragments
            .iter()
            .enumerate()
            .filter(|(_, f)| region.contains_center(f))
            .collect();

        if indexed.is_empty() {
            continue;
        }

        let refs: Vec<&TextFragment> = indexed.iter().map(|(_, f)| *f).collect();

        let col_bounds = find_column_bounds(&refs, *region);
        let row_bounds = find_row_bounds(&refs, structural_paths, *region);

        if col_bounds.len() < 2 || row_bounds.len() < 2 {
            continue;
        }

        let table = build_table(&refs, &col_bounds, &row_bounds, region.top);

        if table.rows.len() < 2 {
            continue;
        }

        for (idx, _) in &indexed {
            claimed[*idx] = true;
        }

        table_blocks.push(ContentBlock::Table(table));
    }

    let remaining: Vec<TextFragment> = fragments
        .into_iter()
        .enumerate()
        .filter_map(|(i, f)| if claimed[i] { None } else { Some(f) })
        .collect();

    (remaining, table_blocks)
}

fn find_table_regions(
    paths: &[(f32, f32, f32, f32)],
    fragments: &[TextFragment],
    page_width: f32,
) -> Vec<Rect> {
    let mut h_lines: Vec<Rect> = Vec::new();
    let mut v_lines: Vec<Rect> = Vec::new();
    let mut cell_rects: Vec<Rect> = Vec::new();

    for &(left, top, right, bottom) in paths {
        let r = Rect::new(left, top, right, bottom);

        if r.height() < 3.0 && r.width() > 20.0 {
            h_lines.push(r);
        } else if r.width() < 3.0 && r.height() > 20.0 {
            v_lines.push(r);
        } else if r.width() > 15.0 && r.height() > 10.0 {
            cell_rects.push(r);
        }
    }

    if cell_rects.len() >= 4 {
        let regions = cluster_into_regions(cell_rects, 5.0, 4);
        if !regions.is_empty() {
            return regions;
        }
    }

    if h_lines.len() >= 2 && v_lines.len() >= 2 {
        if let Some(region) = find_grid_region(&h_lines, &v_lines) {
            return vec![region];
        }
    }

    find_column_aligned_regions(fragments, page_width)
}

fn cluster_into_regions(rects: Vec<Rect>, tolerance: f32, min_count: usize) -> Vec<Rect> {
    let mut clusters: Vec<(Rect, usize)> = Vec::new();

    'rect: for rect in rects {
        for (bbox, count) in clusters.iter_mut() {
            if bbox.overlaps_or_adjacent(rect, tolerance) {
                *bbox = bbox.union(rect);
                *count += 1;
                continue 'rect;
            }
        }
        clusters.push((rect, 1));
    }

    clusters
        .into_iter()
        .filter(|(_, count)| *count >= min_count)
        .map(|(bbox, _)| bbox)
        .collect()
}

fn find_grid_region(h_lines: &[Rect], v_lines: &[Rect]) -> Option<Rect> {
    let v_x_min = v_lines.iter().map(|r| r.left).fold(f32::MAX, f32::min);
    let v_x_max = v_lines.iter().map(|r| r.right).fold(f32::MIN, f32::max);
    let v_y_min = v_lines.iter().map(|r| r.bottom).fold(f32::MAX, f32::min);
    let v_y_max = v_lines.iter().map(|r| r.top).fold(f32::MIN, f32::max);

    // Only consider horizontal lines whose centre falls within the Y span of
    // the vertical lines. Section-heading underlines above or below the table
    // are also thin horizontal paths; without this filter they expand the
    // detected region to include content that isn't part of the table.
    let relevant_h: Vec<Rect> = h_lines
        .iter()
        .copied()
        .filter(|r| {
            let y_mid = (r.top + r.bottom) / 2.0;
            y_mid >= v_y_min && y_mid <= v_y_max
        })
        .collect();

    if relevant_h.len() < 2 {
        return None;
    }

    let h_x_min = relevant_h.iter().map(|r| r.left).fold(f32::MAX, f32::min);
    let h_x_max = relevant_h.iter().map(|r| r.right).fold(f32::MIN, f32::max);
    let h_y_min = relevant_h.iter().map(|r| r.bottom).fold(f32::MAX, f32::min);
    let h_y_max = relevant_h.iter().map(|r| r.top).fold(f32::MIN, f32::max);

    let x_overlap = h_x_min.max(v_x_min) < h_x_max.min(v_x_max);
    let y_overlap = h_y_min.max(v_y_min) < h_y_max.min(v_y_max);

    if !x_overlap || !y_overlap {
        return None;
    }

    Some(Rect {
        left: h_x_min.min(v_x_min),
        top: h_y_max.max(v_y_max),
        right: h_x_max.max(v_x_max),
        bottom: h_y_min.min(v_y_min),
    })
}

fn find_column_aligned_regions(fragments: &[TextFragment], page_width: f32) -> Vec<Rect> {
    if fragments.len() < 6 {
        return Vec::new();
    }

    let row_tolerance = 5.0;
    let mut row_tops: Vec<f32> = Vec::new();
    let mut frag_row_id: Vec<usize> = Vec::with_capacity(fragments.len());

    for frag in fragments {
        let id = match row_tops
            .iter()
            .position(|&y| (y - frag.top).abs() < row_tolerance)
        {
            Some(id) => id,
            None => {
                let id = row_tops.len();
                row_tops.push(frag.top);
                id
            }
        };
        frag_row_id.push(id);
    }

    let total_rows = row_tops.len();
    if total_rows < 3 {
        return Vec::new();
    }

    let x_tolerance = 8.0;
    let mut x_bands: Vec<(f32, Vec<usize>)> = Vec::new();

    for (frag_idx, frag) in fragments.iter().enumerate() {
        let row = frag_row_id[frag_idx];
        match x_bands
            .iter_mut()
            .find(|(x, _)| (x - frag.left).abs() < x_tolerance)
        {
            Some((_, rows)) => {
                if !rows.contains(&row) {
                    rows.push(row);
                }
            }
            None => x_bands.push((frag.left, vec![row])),
        }
    }

    let min_rows = 3.min(total_rows);
    let strong_xs: Vec<f32> = x_bands
        .iter()
        .filter(|(_, rows)| rows.len() >= min_rows)
        .map(|(x, _)| *x)
        .collect();

    if strong_xs.len() < 2 {
        return Vec::new();
    }

    if strong_xs.len() == 2 {
        let left_edge = fragments.iter().map(|f| f.left).fold(f32::MAX, f32::min);
        let right_edge = fragments.iter().map(|f| f.right).fold(f32::MIN, f32::max);
        let gap = (strong_xs[0] - strong_xs[1]).abs();
        let bbox_width = right_edge - left_edge;
        if gap > page_width * 0.10 && bbox_width > page_width * 0.85 {
            return Vec::new();
        }
    }

    let table_frags: Vec<&TextFragment> = fragments
        .iter()
        .filter(|f| strong_xs.iter().any(|x| (x - f.left).abs() < x_tolerance))
        .collect();

    let left = table_frags.iter().map(|f| f.left).fold(f32::MAX, f32::min);
    let right = table_frags.iter().map(|f| f.right).fold(f32::MIN, f32::max);
    let top = table_frags.iter().map(|f| f.top).fold(f32::MIN, f32::max);
    let bottom = table_frags
        .iter()
        .map(|f| f.bottom)
        .fold(f32::MAX, f32::min);

    vec![Rect {
        left: left - 3.0,
        top: top + 3.0,
        right: right + 3.0,
        bottom: bottom - 3.0,
    }]
}

fn find_column_bounds(fragments: &[&TextFragment], region: Rect) -> Vec<(f32, f32)> {
    let x_tolerance = 8.0;
    let mut col_starts: Vec<f32> = Vec::new();

    for frag in fragments {
        if col_starts
            .iter()
            .all(|&x| (x - frag.left).abs() >= x_tolerance)
        {
            col_starts.push(frag.left);
        }
    }

    col_starts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    if col_starts.len() < 2 {
        return Vec::new();
    }

    col_starts
        .iter()
        .enumerate()
        .map(|(i, &start)| {
            let col_left = if i == 0 {
                region.left
            } else {
                (col_starts[i - 1] + start) / 2.0
            };
            let col_right = if i + 1 < col_starts.len() {
                (start + col_starts[i + 1]) / 2.0
            } else {
                region.right
            };
            (col_left, col_right)
        })
        .collect()
}

fn find_row_bounds(
    fragments: &[&TextFragment],
    paths: &[(f32, f32, f32, f32)],
    region: Rect,
) -> Vec<(f32, f32)> {
    let mut h_ys: Vec<f32> = paths
        .iter()
        .filter_map(|&(left, top, right, bottom)| {
            let r = Rect::new(left, top, right, bottom);
            let y_mid = (r.top + r.bottom) / 2.0;
            if r.height() < 3.0 && r.width() > 20.0 && y_mid >= region.bottom && y_mid <= region.top
            {
                Some(y_mid)
            } else {
                None
            }
        })
        .collect();

    if h_ys.len() >= 2 {
        h_ys.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        h_ys.dedup_by(|a, b| (*b - *a).abs() < 3.0);

        let rows: Vec<(f32, f32)> = h_ys.windows(2).map(|w| (w[1], w[0])).collect();

        if !rows.is_empty() {
            return rows;
        }
    }

    let y_tolerance = 5.0;
    let mut row_tops: Vec<f32> = Vec::new();

    for frag in fragments {
        if row_tops
            .iter()
            .all(|&y| (y - frag.top).abs() >= y_tolerance)
        {
            row_tops.push(frag.top);
        }
    }

    row_tops.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    row_tops
        .iter()
        .enumerate()
        .map(|(i, &top)| {
            let row_bottom = if i + 1 < row_tops.len() {
                (top + row_tops[i + 1]) / 2.0
            } else {
                region.bottom
            };
            (row_bottom, top)
        })
        .collect()
}

fn build_table(
    fragments: &[&TextFragment],
    col_bounds: &[(f32, f32)],
    row_bounds: &[(f32, f32)],
    y_position: f32,
) -> Table {
    let n_cols = col_bounds.len();
    let n_rows = row_bounds.len();

    let mut cell_frags: Vec<Vec<Vec<&TextFragment>>> = vec![vec![Vec::new(); n_cols]; n_rows];
    // Text pieces from spanning fragments, stored alongside their style flags.
    let mut cell_extra: Vec<Vec<Vec<(String, bool, bool, bool)>>> =
        vec![vec![Vec::new(); n_cols]; n_rows];

    for frag in fragments {
        let cy = (frag.top + frag.bottom) / 2.0;

        let col = col_bounds
            .iter()
            .position(|&(l, r)| frag.left >= l - 5.0 && frag.left < r);
        let row = row_bounds.iter().position(|&(b, t)| cy >= b && cy < t);

        if let (Some(c), Some(r)) = (col, row) {
            let col_right = col_bounds[c].1;
            if frag.right > col_right + 10.0 && c + 1 < n_cols && frag.text.contains(' ') {
                let (left_text, right_text) =
                    split_text_at_boundary(&frag.text, frag.left, frag.right, col_right);
                if !left_text.is_empty() {
                    cell_extra[r][c].push((
                        left_text,
                        frag.is_bold,
                        frag.is_italic,
                        frag.is_underlined,
                    ));
                }
                if !right_text.is_empty() {
                    cell_extra[r][c + 1].push((
                        right_text,
                        frag.is_bold,
                        frag.is_italic,
                        frag.is_underlined,
                    ));
                }
            } else {
                cell_frags[r][c].push(frag);
            }
        }
    }

    let rows: Vec<TableRow> = (0..n_rows)
        .map(|r| TableRow {
            cells: (0..n_cols)
                .map(|c| {
                    let frags = &cell_frags[r][c];
                    let extras = &cell_extra[r][c];
                    let mut text = merge_cell_fragments(frags);
                    for (extra_text, _, _, _) in extras {
                        if !text.is_empty() {
                            text.push(' ');
                        }
                        text.push_str(extra_text);
                    }
                    let is_bold =
                        frags.iter().any(|f| f.is_bold) || extras.iter().any(|(_, b, _, _)| *b);
                    let is_italic =
                        frags.iter().any(|f| f.is_italic) || extras.iter().any(|(_, _, i, _)| *i);
                    let is_underlined = frags.iter().any(|f| f.is_underlined)
                        || extras.iter().any(|(_, _, _, u)| *u);
                    TableCell {
                        text: text.trim().to_string(),
                        is_bold,
                        is_italic,
                        is_underlined,
                        col_span: 1,
                        row_span: 1,
                    }
                })
                .collect(),
        })
        .filter(|row| row.cells.iter().any(|c| !c.text.is_empty()))
        .collect();

    Table {
        rows,
        col_count: n_cols,
        y_position,
    }
}

fn split_text_at_boundary(
    text: &str,
    frag_left: f32,
    frag_right: f32,
    boundary_x: f32,
) -> (String, String) {
    let frag_width = frag_right - frag_left;
    if frag_width <= 0.0 {
        return (text.to_string(), String::new());
    }
    let split_fraction = (boundary_x - frag_left).max(0.0) / frag_width;
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 2 {
        return (text.to_string(), String::new());
    }
    let total_chars = words.iter().map(|w| w.len()).sum::<usize>() + (words.len() - 1);
    let mut cumulative = 0usize;
    let mut split_idx = words.len();
    for (i, word) in words.iter().enumerate() {
        if i > 0 && cumulative as f32 / total_chars as f32 >= split_fraction {
            split_idx = i;
            break;
        }
        cumulative += word.len() + 1;
    }
    if split_idx >= words.len() {
        return (text.to_string(), String::new());
    }
    (words[..split_idx].join(" "), words[split_idx..].join(" "))
}

fn merge_cell_fragments(frags: &[&TextFragment]) -> String {
    if frags.is_empty() {
        return String::new();
    }

    let y_tol = 4.0;
    let mut lines: Vec<Vec<&TextFragment>> = Vec::new();

    for frag in frags {
        match lines
            .iter_mut()
            .find(|line| line.iter().any(|f| (f.top - frag.top).abs() < y_tol))
        {
            Some(line) => line.push(frag),
            None => lines.push(vec![frag]),
        }
    }

    for line in lines.iter_mut() {
        line.sort_by(|a, b| {
            a.left
                .partial_cmp(&b.left)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    lines.sort_by(|a, b| {
        let a_top = a.iter().map(|f| f.top).fold(f32::MIN, f32::max);
        let b_top = b.iter().map(|f| f.top).fold(f32::MIN, f32::max);
        b_top
            .partial_cmp(&a_top)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    lines
        .iter()
        .map(|line| {
            line.iter()
                .map(|f| f.text.as_str())
                .collect::<String>()
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
