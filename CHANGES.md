# CHANGES

A walkthrough of every code modification made to `server/src/pdf_inference/reconstruct.rs` while fixing PDF parsing for `qemu_long_pdf.pdf` and the resume PDF, in the order the changes happened. Each entry covers what was added, removed, or changed and why.

For the underlying concepts (subsetted fonts, TJ kerning, baselines, etc.), see [COOL_STUFF.md](./COOL_STUFF.md).

---

## Round 1 — fix per-glyph fragments and broken alignment

**Problem.** `qemu_long_pdf.pdf` uses a subsetted font with per-glyph TJ positioning, so PDFium emits one text object per letter. The downstream pipeline assumed text objects were words/runs. Two fallout effects:

1. Performance: hundreds of fragments per page where the simple PDF had dozens.
2. Alignment detection broke. Alignment was computed *per fragment* against page width — a 1-glyph-wide fragment is never centered, so titles, subtitles, epigraphs all collapsed to `Left` and were misclassified.

### Removed: `alignment` field from `TextFragment`

```rust
pub struct TextFragment {
    pub text: String,
    pub font_name: String,
    ...
    pub bottom: f32,
-   pub alignment: TextAlign,
}
```

A per-glyph `alignment` value carries no real information. The page-centering question only makes sense at line scope (where you can actually measure left and right margins of the *content*, not of a single letter). I deleted the field rather than leaving a vestigial one — it had no users outside `reconstruct.rs`.

### Added: `alignment` field on `TextLine`

```rust
pub struct TextLine {
    pub fragments: Vec<TextFragment>,
    pub top: f32,
    pub bottom: f32,
+   pub alignment: TextAlign,
}
```

Stored on the line because it's a line-scope property. Computed once when the line is finalized rather than on every `alignment()` call.

### Changed: `TextLine::alignment()` from "first-fragment lookup" to getter

```rust
pub fn alignment(&self) -> TextAlign {
-   self.fragments
-       .iter()
-       .find(|f| !f.text.trim().is_empty())
-       .map(|f| f.alignment.clone())
-       .unwrap_or(TextAlign::Left)
+   self.alignment.clone()
}
```

Used to scan fragments and read the first one's alignment. Now just returns the stored line-level value.

### Removed: per-fragment alignment computation in `extract_fragments`

```rust
- let page_width = page.width().value;
  ...
  if let Ok(bounds) = text_obj.bounds() {
      let left = bounds.left().value;
      ...
-     let width = bounds.width().value;
-
-     let center_margin = (page_width - width) / 2.0;
-     let left_diff = (center_margin - left).abs();
-     let alignment = if left_diff <= 3.0 {
-         TextAlign::Center
-     } else {
-         TextAlign::Left
-     };
```

`page_width` and the alignment block were the only reason `extract_fragments` knew about the page. Moving alignment to line scope made both unnecessary. Cleaner extraction stage that does only one thing — pull text objects out of PDFium.

### Added: `merge_line_fragments` and `can_merge`

```rust
fn can_merge(prev: &TextFragment, next: &TextFragment) -> bool {
    prev.font_name == next.font_name
        && (prev.font_size - next.font_size).abs() < 0.5
        && prev.is_bold == next.is_bold
        && prev.is_italic == next.is_italic
        && prev.is_underlined == next.is_underlined
}

fn merge_line_fragments(fragments: Vec<TextFragment>) -> Vec<TextFragment> {
    // Walks fragments left-to-right, folding adjacent same-style ones
    // into one. Inserts a space iff gap > 0.2 × font_size.
}
```

This is the consolidation pass: for per-glyph PDFs, it folds individual letters back into word/run-level fragments so the rest of the pipeline (line grouping, classification, block merging) sees the same shape of data it did for the simple PDF.

The space-insertion threshold of `0.2 × font_size` is empirical. Inter-character kerning is well below that; inter-word spacing is well above it. Tested against actual inter-glyph distances from the qemu PDF — every word-internal gap was below the threshold, every inter-word gap was above.

`can_merge` is intentionally strict on style. Two fragments only fold if every style attribute matches. Mixing styles inside one fragment would lose information needed downstream (e.g., a partly-bold line classifying as `Heading`).

### Added: `line_alignment` helper

```rust
fn line_alignment(fragments: &[TextFragment], page_width: f32) -> TextAlign {
    let leftmost = fragments.iter().map(|f| f.left).fold(f32::MAX, f32::min);
    let rightmost = fragments.iter().map(|f| f.right).fold(f32::MIN, f32::max);
    let line_width = rightmost - leftmost;
    let center_margin = (page_width - line_width) / 2.0;
    let left_diff = (center_margin - leftmost).abs();
    if left_diff <= 3.0 { TextAlign::Center } else { TextAlign::Left }
}
```

The line-scope replacement for the per-fragment alignment block we deleted. Computes against the *line's combined extent* — `min(left)` to `max(right)` — rather than any single fragment. A centered title comes out as `Center` even when it's split into many same-baseline glyph fragments.

(This version had a subtle bug — see Round 3 — but Round 1 is just about getting alignment off the fragment.)

### Changed: `group_into_lines` now takes `page_width` and runs the merge + alignment pass

```rust
- pub fn group_into_lines(mut fragments: Vec<TextFragment>) -> Vec<TextLine> {
+ pub fn group_into_lines(mut fragments: Vec<TextFragment>, page_width: f32) -> Vec<TextLine> {
```

A small closure was added inside the function:

```rust
let finish_line = |frags, top, bottom| -> TextLine {
    let mut frags = frags;
    frags.sort_by(|a, b| a.left.partial_cmp(&b.left).unwrap_or(...));
    let merged = merge_line_fragments(frags);
    let alignment = line_alignment(&merged, page_width);
    TextLine { fragments: merged, top, bottom, alignment }
};
```

Three responsibilities at one point: sort fragments left-to-right, run the consolidation pass, and compute alignment. Centralizing it kept the loop readable and made the "what does a finished line look like?" answer obvious in one place.

### Changed: `reconstruct_page` passes `page_width` through

```rust
- let lines = group_into_lines(fragments);
+ let lines = group_into_lines(fragments, page.width().value);
```

Just plumbing.

---

## Round 2 — fix descenders and punctuation splitting off into ghost lines

**Problem.** After Round 1, the qemu PDF's title `USB Device Redirection for qemu-rdp` came out split across two lines:

```
[Heading] USB De ice Redirection for d
[Heading] v qemu-r p
```

And paragraphs grew bunched commas at the end:

```
... audio clipboard console keyboard , , , , , mouse multitouch ...
```

The line-grouping rule used vertical-midpoint distance. For a 20pt heading, a cap glyph's midpoint is ~5pt above a descender's midpoint (because their bboxes have different shapes around a shared baseline). The threshold of 3pt put `q`, `p`, `v`, etc. on a phantom line. Same story for commas, whose midpoints sit well below cap-height letters.

### Changed: same-line test — midpoint distance → bbox vertical overlap

```rust
- let frag_mid = (frag.top + frag.bottom) / 2.0;
- let line_mid = (current_top + current_bottom) / 2.0;
- let on_same_line = (frag_mid - line_mid).abs() <= 3.0;
+ let overlap = current_top.min(frag.top) - current_bottom.max(frag.bottom);
+ let on_same_line = overlap > 0.5;
```

Bbox overlap is the right invariant for "are these glyphs on the same line?" — within a visual line, every glyph's bbox heavily overlaps every other glyph's bbox, regardless of cap/x-height/descender/punctuation. Adjacent stacked lines have *negative* overlap (the inter-line gap), so they don't accidentally join.

The 0.5pt threshold over a strict `> 0` is just a cushion for floating-point noise from PDFium's bbox calculations.

This fixed the qemu PDF cleanly. It later turned out to fail on multi-column layouts (Round 3).

---

## Round 3 — fix multi-column resume layouts

**Problem.** A two-column resume gave two new symptoms:

1. The header `Abass Azeez Afolarin` (left, 20pt) absorbed all three stacked right-column lines (`+234... | email`, `github | linkedin`, `thefola.dev | Nigeria`) into one mangled line, because the 20pt bbox vertically overlapped three 9pt lines.
2. Every "Job Title……Date" row got classified as `[Subtitle]` (centered) because the row spans both content margins symmetrically.

### Changed: same-line test — bbox overlap → baseline match + font-size compatibility

```rust
- let overlap = current_top.min(frag.top) - current_bottom.max(frag.bottom);
- let on_same_line = overlap > 0.5;
+ let baseline_diff = (current_baseline - frag.bottom).abs();
+ let min_fs = current_max_font_size.min(frag.font_size).max(0.1);
+ let max_fs = current_max_font_size.max(frag.font_size);
+ let baseline_compatible = baseline_diff < min_fs * 0.4;
+ let font_compatible = max_fs / min_fs < 1.5;
+ let on_same_line = baseline_compatible && font_compatible;
```

Two ideas:

- **Baseline match.** For non-descender glyphs, `bbox.bottom ≈ baseline`. So tracking `max(bottom)` across a line's fragments converges on the line's true baseline. New fragments must sit close to that baseline (within `0.4 × font_size`, which is wide enough for descenders' ~21% drop and commas' ~1pt drop, narrow enough to separate adjacent stacked lines).
- **Font-size compatibility.** Even if baselines accidentally align (which happens in multi-column layouts where the small right-column text is centered against a big left-column header), refuse to merge fragments whose font sizes differ by more than 1.5×. This is the new guard that keeps the 20pt name from absorbing the 9pt right column.

The `.max(0.1)` on `min_fs` is a divide-by-zero guard. PDFium can return a 0.0 font size for invisible text objects.

### Added: `current_baseline` and `current_max_font_size` trackers

```rust
+ let mut current_baseline = fragments[0].bottom;
+ let mut current_max_font_size = fragments[0].font_size;
```

Updated when a fragment joins the line:

```rust
+ if frag.bottom > current_baseline { current_baseline = frag.bottom; }
+ if frag.font_size > current_max_font_size { current_max_font_size = frag.font_size; }
```

And reset when starting a new line:

```rust
+ current_baseline = frag.bottom;
+ current_max_font_size = frag.font_size;
```

`current_baseline` is `max(bottom)` (baseline = highest bottom seen, since non-descenders sit on it and descenders dip below). `current_max_font_size` is the running max so the ratio test stays meaningful as more fragments join.

The existing `current_top`/`current_bottom` were *not* removed — they still describe the line's bbox, which paragraph-break detection in `merge_into_blocks` reads. The new fields live alongside.

### Changed: `line_alignment` rejects `Center` when there's a big internal gap

```rust
+ let max_internal_gap = fragments
+     .windows(2)
+     .map(|pair| pair[1].left - pair[0].right)
+     .fold(0.0_f32, f32::max);
+ let has_column_gap = max_internal_gap > page_width * 0.1;
+
- if left_diff <= 3.0 {
+ if left_diff <= 3.0 && !has_column_gap {
      TextAlign::Center
  } else {
      TextAlign::Left
  }
```

The original test (`outer margins symmetric within 3pt → Center`) gave false positives on rows like `Job Title    Date` that span both margins. Real centered text is contiguous; a row with an empty middle is a multi-column layout, not a centered title.

The 10% of page width threshold (~61pt on US Letter) is a clean separator — comfortably wider than any natural inter-word space, comfortably narrower than any deliberate column gap.

`fragments.windows(2)` is safe here because `finish_line` already sorted fragments left-to-right before calling `line_alignment`.

---

## Round 4 — fix cluster fragmentation in interleaved iteration; loosen two thresholds

**Problem.** After Round 3, the resume PDF still misbehaved in three ways:

1. The name `Abass Azeez Afolarin` came out as `Ab A Af l i . ass zeez o ar n` — caps and ascender letters split off from x-height letters of the same word.
2. Commas regressed to the Bug 2 symptom: bunched at the end of paragraphs (`audio clipboard console keyboard , , , , ,`).
3. `Nigeria` came out as `N igeria`.

The first two are the same bug. The Round 1–3 grouping loop kept **one active line at a time**: it sorted fragments by top descending and walked them, accumulating into the active line and closing it whenever a fragment didn't fit. In a multi-column layout, fragments from two different visual lines interleave in top-order:

```
top=754 — header caps (font 20, baseline 740)
top=752 — right-column caps (font 9, baseline 745)
top=750 — header x-height AND right-column x-height letters
```

Each time the iteration hit a fragment that didn't fit the active line, the loop closed and reopened — but the fragment that *did* belong to the line we just closed could be three iterations away. So both columns got fragmented into many tiny sub-lines, which then got concatenated by `merge_into_blocks` in the wrong order.

The third issue (`N igeria`) is unrelated and falls out of the ink-vs-advance gap for narrow sans-serif letters.

### Added: `LineCluster` struct

```rust
struct LineCluster {
    fragments: Vec<TextFragment>,
    baseline: f32,
    max_font_size: f32,
    top: f32,
    bottom: f32,
}

impl LineCluster {
    fn new(frag: TextFragment) -> Self { ... }
    fn accepts(&self, frag: &TextFragment) -> bool { ... }
    fn absorb(&mut self, frag: TextFragment) { ... }
}
```

Replaces the loose tuple of `current_*` mutable variables in the old loop. Each cluster owns its own state (fragments, baseline, font size, bbox), and the methods (`accepts`, `absorb`, `new`) keep the rest of the function readable.

### Changed: `group_into_lines` loop body — single-active-line → cluster lookup

```rust
- for frag in fragments.iter().skip(1) {
-     let baseline_diff = (current_baseline - frag.bottom).abs();
-     ...
-     if on_same_line {
-         current_fragments.push(frag.clone());
-         ...
-     } else {
-         lines.push(finish_line(...));
-         current_fragments = vec![frag.clone()];
-         ...
-     }
- }

+ for frag in fragments {
+     match clusters.iter().position(|c| c.accepts(&frag)) {
+         Some(idx) => clusters[idx].absorb(frag),
+         None => clusters.push(LineCluster::new(frag)),
+     }
+ }
```

Each fragment hunts for a matching cluster anywhere in the active set — not just the most-recent one. Fragments from interleaved lines now accrete onto their respective clusters regardless of arrival order. After all fragments are placed, clusters get sorted by `top` descending into reading order, then converted into `TextLine`s exactly as before (sort fragments by left, run `merge_line_fragments`, compute `line_alignment`).

The cost is O(n × c) where `c` is the number of distinct lines on a page. For typical documents `c` is small enough (~30–80) that this is fine; if it ever becomes a hot path, sort fragments by baseline first and you can do the cluster lookup in O(log c) per fragment.

### Changed: baseline tolerance `0.4 × min_fs` → `0.5 × min_fs`

```rust
- baseline_diff < min_fs * 0.4
+ baseline_diff < min_fs * 0.5
```

The 0.4 cutoff was borderline for two cases:

- Commas in some sans-serif fonts whose comma bbox dips a bit further below the baseline than I assumed in Round 3.
- Cap-vs-x-height bbox-bottom variation in fonts whose lowercase letters report a slightly different `bottom` than caps (some PDF generators add small per-glyph padding that isn't strictly at the baseline).

0.5 covers them comfortably. Adjacent stacked lines have baselines differing by a full line-height (~1.2 × em), which is well outside even the looser tolerance.

### Changed: space-insertion threshold `0.2 × font_size` → `0.3 × font_size`

Both call sites — `merge_line_fragments` and `TextLine::merged_text` — were updated to keep them in sync.

```rust
- let threshold = prev.font_size.max(frag.font_size) * 0.2;
+ let threshold = prev.font_size.max(frag.font_size) * 0.3;
```

The reason is structural: PDFium's `bounds()` returns the **ink bbox**, not the **advance bbox**. The horizontal gap between adjacent same-word glyphs can be wider than you'd expect because the ink of letter `N` ends well before the right edge of its advance, and letter `i`'s ink sits in the middle of its advance box. For 9pt Arial, that ink gap is ~1.4pt; for some other sans-serif at 9pt, it can hit 2.0pt — over the old 1.8pt threshold, which is what produced `N igeria`.

`0.3 × em` is comfortably above any natural ink-gap-within-a-word and comfortably below a real space character (which advances by ~`0.28 × em` plus surrounding inter-letter spacing). See COOL_STUFF.md's "Ink width vs advance width" section for the longer story.

---

## Round 5 — fix apostrophes (and any above-baseline punctuation) splitting off

**Problem.** After Round 4, `I'm` and `I've` came out as `I m` and `I ve`, with the apostrophes piled up later in the paragraph as `' '`. Same shape as the original Round 2 comma bug, but flipped vertically: apostrophes float *above* the baseline (bottom ≈ baseline + 0.5 × em) where commas float just below it.

The Round 4 rule was a baseline-distance test: `baseline_diff < min_fs * 0.5`. For 9pt apostrophes:

```
baseline_diff = (line.baseline) - (apostrophe.bottom)  ≈ -4.5
abs ≈ 4.5
tolerance = 9 * 0.5 = 4.5
4.5 < 4.5  →  false  →  rejected
```

Right at the threshold. And the threshold couldn't simply be bumped, because:

- Pushing it to 0.7 × em accepts apostrophes but starts merging adjacent stacked lines (~12pt baseline gap minus tolerance gets uncomfortably close).
- The baseline rule has an asymmetry built in: when an apostrophe arrived first in iteration order (its top is similar to caps' tops, so sort-by-top-desc could place it before regular letters), it seeded a cluster with `baseline = apostrophe.bottom`. Subsequent letters had `bottom` 4.5pt below that polluted baseline and then *they* couldn't join.

The asymmetry is the deeper issue. Any same-line rule based on a single reference point (baseline, midpoint, top) handles one flavour of punctuation and fails on the other.

### Changed: same-line test — baseline distance → bbox vertical overlap

```rust
- fn accepts(&self, frag: &TextFragment) -> bool {
-     let baseline_diff = (self.baseline - frag.bottom).abs();
-     let min_fs = self.max_font_size.min(frag.font_size).max(0.1);
-     let max_fs = self.max_font_size.max(frag.font_size);
-     baseline_diff < min_fs * 0.5 && max_fs / min_fs < 1.5
- }

+ fn accepts(&self, frag: &TextFragment) -> bool {
+     let overlap = self.top.min(frag.top) - self.bottom.max(frag.bottom);
+     let min_fs = self.max_font_size.min(frag.font_size).max(0.1);
+     let max_fs = self.max_font_size.max(frag.font_size);
+     overlap > 0.5 && max_fs / min_fs < 1.5
+ }
```

This is Round 2's bbox-overlap rule, with Round 3's font-size-ratio cap layered on top. Bbox overlap is symmetric — it doesn't care whether a glyph sits above the baseline (apostrophe, quote) or below (comma, period, descender). The line's combined `top..bottom` band always overlaps every member glyph's band by at least a few points; adjacent stacked lines have *negative* overlap (the inter-line gap).

The font-ratio cap is what makes this safe in multi-column layouts — the only reason we abandoned bbox overlap in Round 3. A 20pt header line's bbox vertically overlaps a stack of 9pt right-column lines' bboxes, but `max_fs / min_fs = 2.22 > 1.5` rejects that pairing. Letter-vs-punctuation pairings always have ratio = 1, so they sail through.

### Removed: `LineCluster::baseline` field

```rust
  struct LineCluster {
      fragments: Vec<TextFragment>,
-     baseline: f32,
      max_font_size: f32,
      top: f32,
      bottom: f32,
  }
```

The new `accepts` test reads only `top`, `bottom`, and `max_font_size`. The baseline tracker became dead code; it was deleted along with the corresponding update in `absorb()` and the initialization in `new()`.

This is also a small simplification: the baseline-via-`max(bottom)` heuristic was the most fragile part of the Round 4 design (apostrophes broke it; comma-only lines broke it differently; we'd have hit something else eventually). Removing it means there's one fewer running estimate to worry about.

---

## Files touched

- `server/src/pdf_inference/reconstruct.rs` — all the changes above.
- `server/src/pdf_inference/mod.rs` — minor: added a debug print of merged lines in `reconstruct_page` (replaced the old per-fragment dump). No behavior change.

## Files NOT touched

- `server/src/pdf_inference/mod.rs::infer_pdf_semantics` is still hardcoded to load `test/qemu_pdf.pdf`. This was deliberate — we wanted to test against a known input. Switch the path or restore `load_pdf_from_byte_slice(pdf_bytes, None)` when re-wiring to the HTTP layer.
- `merge_into_blocks` and `classify_line` were left alone. They consume `TextLine`s; once line grouping produces correctly-clustered lines with correct alignment, classification works without modification.
