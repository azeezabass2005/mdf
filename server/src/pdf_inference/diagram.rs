use base64::{Engine as _, engine::general_purpose};
use image::ImageFormat;
use pdfium_render::prelude::*;
use std::io::Cursor;

#[derive(Clone, Copy)]
struct Bbox {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl Bbox {
    fn from_tuple((l, t, r, b): (f32, f32, f32, f32)) -> Self {
        Self {
            left: l.min(r),
            top: t.max(b),
            right: r.max(l),
            bottom: b.min(t),
        }
    }

    fn width(self) -> f32 {
        self.right - self.left
    }

    fn height(self) -> f32 {
        self.top - self.bottom
    }

    fn union(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.max(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.min(other.bottom),
        }
    }

    fn overlaps_within(self, other: Self, slack: f32) -> bool {
        self.left - slack <= other.right
            && self.right + slack >= other.left
            && self.bottom - slack <= other.top
            && self.top + slack >= other.bottom
    }

    fn contains_point(self, x: f32, y: f32) -> bool {
        x >= self.left && x <= self.right && y >= self.bottom && y <= self.top
    }
}

pub fn detect_diagram_regions(
    paths: &[(f32, f32, f32, f32)],
    excluded: Option<(f32, f32, f32, f32)>,
) -> Vec<(f32, f32, f32, f32)> {
    let excluded_bbox = excluded.map(Bbox::from_tuple);

    let candidates: Vec<Bbox> = paths
        .iter()
        .map(|&p| Bbox::from_tuple(p))
        .filter(|p| match excluded_bbox {
            Some(eb) => {
                let cx = (p.left + p.right) / 2.0;
                let cy = (p.top + p.bottom) / 2.0;
                !eb.contains_point(cx, cy)
            }
            None => true,
        })
        .collect();

    if candidates.len() < 6 {
        return Vec::new();
    }

    let n = candidates.len();
    let mut parent: Vec<usize> = (0..n).collect();

    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }

    let slack = 12.0;
    for i in 0..n {
        for j in (i + 1)..n {
            if candidates[i].overlaps_within(candidates[j], slack) {
                let ri = root(&mut parent, i);
                let rj = root(&mut parent, j);
                if ri != rj {
                    parent[ri] = rj;
                }
            }
        }
    }

    let mut groups: Vec<(Bbox, usize)> = Vec::new();
    let mut group_idx: Vec<Option<usize>> = vec![None; n];

    for i in 0..n {
        let r = root(&mut parent, i);
        match group_idx[r] {
            Some(gi) => {
                groups[gi].0 = groups[gi].0.union(candidates[i]);
                groups[gi].1 += 1;
            }
            None => {
                group_idx[r] = Some(groups.len());
                groups.push((candidates[i], 1));
            }
        }
    }

    let padding = 25.0;
    groups
        .into_iter()
        .filter(|(b, c)| *c >= 6 && b.width() >= 100.0 && b.height() >= 60.0)
        .map(|(b, _)| {
            (
                b.left - padding,
                b.top + padding,
                b.right + padding,
                b.bottom - padding,
            )
        })
        .collect()
}

pub fn rasterize_regions(
    page: &PdfPage,
    regions: &[(f32, f32, f32, f32)],
    dpi: f32,
) -> Vec<Option<String>> {
    if regions.is_empty() {
        return Vec::new();
    }

    let scale = dpi / 72.0;
    let page_w_pt = page.width().value;
    let page_h_pt = page.height().value;
    let page_w_px = ((page_w_pt * scale).ceil() as i32).max(1);
    let page_h_px = ((page_h_pt * scale).ceil() as i32).max(1);

    let config = PdfRenderConfig::new()
        .set_target_width(page_w_px)
        .set_target_height(page_h_px);

    let bitmap = match page.render_with_config(&config) {
        Ok(b) => b,
        Err(_) => return regions.iter().map(|_| None).collect(),
    };

    let dynamic = match bitmap.as_image() {
        Ok(img) => img,
        Err(_) => return regions.iter().map(|_| None).collect(),
    };

    let page_img = dynamic.to_rgb8();
    let img_w = page_img.width();
    let img_h = page_img.height();

    regions
        .iter()
        .map(|&(l, t, r, b)| {
            let w_pt = (r - l).max(0.0);
            let h_pt = (t - b).max(0.0);
            if w_pt < 10.0 || h_pt < 10.0 {
                return None;
            }

            let x_px = ((l * scale).max(0.0) as u32).min(img_w);
            let y_px = (((page_h_pt - t) * scale).max(0.0) as u32).min(img_h);
            let mut w_px = (w_pt * scale).ceil() as u32;
            let mut h_px = (h_pt * scale).ceil() as u32;
            w_px = w_px.min(img_w.saturating_sub(x_px));
            h_px = h_px.min(img_h.saturating_sub(y_px));

            if w_px < 10 || h_px < 10 {
                return None;
            }

            let cropped = image::imageops::crop_imm(&page_img, x_px, y_px, w_px, h_px).to_image();

            let cap = 1200u32;
            let dyn_img = image::DynamicImage::ImageRgb8(cropped);
            let dyn_img = if dyn_img.width() > cap || dyn_img.height() > cap {
                dyn_img.thumbnail(cap, cap)
            } else {
                dyn_img
            };

            let mut buf = Cursor::new(Vec::<u8>::new());
            if dyn_img.write_to(&mut buf, ImageFormat::Jpeg).is_err() {
                return None;
            }

            Some(format!(
                "data:image/jpeg;base64,{}",
                general_purpose::STANDARD.encode(buf.into_inner())
            ))
        })
        .collect()
}
