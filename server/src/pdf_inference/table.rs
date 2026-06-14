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
}

pub struct GridLayout {
    region: Rect,
    col_bounds: Vec<(f32, f32)>,
    row_bounds: Vec<(f32, f32)>,
}

impl GridLayout {
    pub fn contains(&self, frag: &TextFragment) -> bool {
        self.region.contains_center(frag)
    }

    pub fn bbox(&self) -> (f32, f32, f32, f32) {
        (
            self.region.left,
            self.region.top,
            self.region.right,
            self.region.bottom,
        )
    }
}

pub fn extract_table(
    fragments: &[TextFragment],
    layout: &GridLayout,
) -> Option<ContentBlock> {
    if fragments.is_empty() {
        return None;
    }
    let refs: Vec<&TextFragment> = fragments.iter().collect();
    let table = build_table(&refs, &layout.col_bounds, &layout.row_bounds, layout.region.top);
    if table.rows.len() < 2 {
        return None;
    }
    Some(ContentBlock::Table(table))
}

pub fn find_grid_layout(paths: &[(f32, f32, f32, f32)]) -> Option<GridLayout> {
    let mut h_lines: Vec<Rect> = Vec::new();
    let mut v_lines: Vec<Rect> = Vec::new();

    for &(left, top, right, bottom) in paths {
        let r = Rect::new(left, top, right, bottom);

        if r.height() < 3.0 && r.width() > 20.0 {
            h_lines.push(r);
        } else if r.width() < 3.0 && r.height() > 20.0 {
            v_lines.push(r);
        }
    }

    if h_lines.len() < 2 || v_lines.len() < 2 {
        return None;
    }

    let v_y_min = v_lines.iter().map(|r| r.bottom).fold(f32::MAX, f32::min);
    let v_y_max = v_lines.iter().map(|r| r.top).fold(f32::MIN, f32::max);
    let v_x_min = v_lines.iter().map(|r| r.left).fold(f32::MAX, f32::min);
    let v_x_max = v_lines.iter().map(|r| r.right).fold(f32::MIN, f32::max);
    let extent_w = v_x_max - v_x_min;
    let extent_h = v_y_max - v_y_min;

    if extent_w <= 0.0 || extent_h <= 0.0 {
        return None;
    }

    let span_ratio = 0.7;

    let spanning_v: Vec<&Rect> = v_lines
        .iter()
        .filter(|r| r.height() >= extent_h * span_ratio)
        .collect();

    let spanning_h: Vec<&Rect> = h_lines
        .iter()
        .filter(|r| {
            let y_mid = (r.top + r.bottom) / 2.0;
            y_mid >= v_y_min - 1.0
                && y_mid <= v_y_max + 1.0
                && r.width() >= extent_w * span_ratio
        })
        .collect();

    let mut v_xs: Vec<f32> = spanning_v.iter().map(|r| (r.left + r.right) / 2.0).collect();
    v_xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v_xs.dedup_by(|a, b| (*b - *a).abs() < 3.0);

    let mut h_ys: Vec<f32> = spanning_h.iter().map(|r| (r.top + r.bottom) / 2.0).collect();
    h_ys.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    h_ys.dedup_by(|a, b| (*b - *a).abs() < 3.0);

    if v_xs.len() < 3 || h_ys.len() < 3 {
        return None;
    }

    let col_bounds: Vec<(f32, f32)> = v_xs.windows(2).map(|w| (w[0], w[1])).collect();
    let row_bounds: Vec<(f32, f32)> = h_ys.windows(2).map(|w| (w[1], w[0])).collect();

    let region = Rect {
        left: *v_xs.first().unwrap(),
        right: *v_xs.last().unwrap(),
        top: *h_ys.first().unwrap(),
        bottom: *h_ys.last().unwrap(),
    };

    Some(GridLayout {
        region,
        col_bounds,
        row_bounds,
    })
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

    for frag in fragments {
        let cx = (frag.left + frag.right) / 2.0;
        let cy = (frag.top + frag.bottom) / 2.0;

        let col = col_bounds.iter().position(|&(l, r)| cx >= l && cx < r);
        let row = row_bounds.iter().position(|&(b, t)| cy >= b && cy < t);

        if let (Some(c), Some(r)) = (col, row) {
            cell_frags[r][c].push(frag);
        }
    }

    let rows: Vec<TableRow> = (0..n_rows)
        .map(|r| TableRow {
            cells: (0..n_cols)
                .map(|c| {
                    let frags = &cell_frags[r][c];
                    let text = merge_cell_fragments(frags);
                    TableCell {
                        text: text.trim().to_string(),
                        is_bold: frags.iter().any(|f| f.is_bold),
                        is_italic: frags.iter().any(|f| f.is_italic),
                        is_underlined: frags.iter().any(|f| f.is_underlined),
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
            let mut s = String::new();
            for (i, f) in line.iter().enumerate() {
                if i > 0 {
                    let prev = line[i - 1];
                    let gap = f.left - prev.right;
                    let font_size = prev.font_size.max(f.font_size);
                    if gap > 0.3 * font_size
                        && !s.ends_with(' ')
                        && !f.text.starts_with(' ')
                    {
                        s.push(' ');
                    }
                }
                s.push_str(&f.text);
            }
            s.trim().to_string()
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
