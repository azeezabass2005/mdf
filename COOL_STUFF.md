# Cool Stuff: PDF Internals We Hit While Building MDF

Notes on the PDF concepts behind the bugs we ran into parsing `qemu_long_pdf.pdf` vs `qemu_pdf.pdf`. Written so future-us doesn't have to relearn this.

## The original symptom

Two PDFs, same source content:

- `qemu_pdf.pdf` — produced by hitting Ctrl+P on the long PDF and printing page 1. Parses cleanly: each fragment is a word or phrase. Fonts: `ArialMT`, `Arial-BoldMT`.
- `qemu_long_pdf.pdf` — the original. Parses badly: each fragment is one letter. Fonts: `AAAAAA+ArialMT`, `BAAAAA+Arial-BoldMT`.

Same words, totally different fragment streams. Everything below explains why.

## Glyphs vs characters

A **character** is the abstract idea — "the letter A", a Unicode codepoint.
A **glyph** is the actual drawn shape — "A in 12pt Arial Bold", a specific picture.

The mapping isn't 1:1:

- One character → many glyphs across fonts/weights/styles.
- One glyph → multiple characters, e.g. the `fi` ligature is *one* glyph that encodes *two* characters.
- Conversely, one character can be drawn with multiple glyphs (combining marks).

Inside a PDF, fonts aren't keyed by Unicode. They're keyed by **glyph IDs** — internal indices into the font's glyph table. The PDF's text-show operators say "draw glyph #47, then #112, then #29." A separate `ToUnicode` table maps glyph IDs back to characters so copy-paste and search work. If that table is missing or wrong, you can read a PDF visually but not extract its text.

## Font subsetting

Embedding a full font in every PDF is wasteful — Arial alone is ~750 KB and the document only uses ~80 of its glyphs. So PDF generators **subset**: include just the glyphs actually used, renumber them, and embed that mini-font.

The convention is to prefix the subsetted font's name with **six uppercase letters and a `+`**:

```
AAAAAA+ArialMT
BAAAAA+Arial-BoldMT
```

The six letters are arbitrary tags (often just incremented per subset in the file). The `+` is the marker. If you see that prefix, the font has been subsetted.

Subsetting itself isn't the bug — it's a hint about how the PDF was generated. Generators that subset aggressively also tend to use per-glyph positioning (see TJ below), and that *is* the bug source.

## Tj vs TJ — the text-show operators

PDF has two main operators for drawing text:

- **`Tj`** (lowercase j) — show one string. The font handles inter-glyph advance widths.

  ```
  (Hello world) Tj
  ```

  Simple, compact. PDFium parses each `Tj` as one text object containing the whole string.

- **`TJ`** (uppercase J) — show an array of strings *with explicit horizontal adjustments between them*.

  ```
  [(A) -80 (V) -50 (e) 30 (...)] TJ
  ```

  Numbers between strings are positioning offsets in thousandths of an em (negative = move left = tighter, positive = move right = looser). This is how PDFs encode kerning and other fine-grained positioning.

When a PDF uses `TJ` arrays with offsets between *every* glyph (which is common for renderers that want pixel-precise output across all viewers), PDFium emits **one text object per glyph**. That's why `text_obj.text()` returned single letters for `qemu_long_pdf.pdf` — every letter was its own `TJ` chunk.

The Ctrl+P-and-print pipeline rerasterized through the OS print stack, which produced a fresh PDF using simple `Tj` runs — one text object per word. Same visual output, totally different operator stream.

## Kerning

Most pairs of letters look fine at default advance widths. Some pairs don't:

- `AV` — the slanted right side of `A` and slanted left side of `V` create a visual gap that looks too wide.
- `To` — the `o` should tuck under the `T`'s arm; default advance leaves them too far apart.
- `Wa`, `Yo`, `Te`, etc.

Fonts ship a **kerning table**: a list of glyph pairs and the adjustment to apply. Negative kerning closes gaps, positive opens them. In a PDF this surfaces as the numbers inside `TJ` arrays.

Kerning is what makes typeset text look right. It's also what triggers per-glyph text objects in PDFium when the generator emits one `TJ` chunk per pair.

## Typography metrics: baseline, cap-height, x-height, ascender, descender

Every line of text has a **baseline** — an invisible horizontal line that the bottoms of most letters sit on. It's the canonical y-coordinate of a line. Two glyphs share a baseline iff they're on the same visual line (modulo superscripts).

Around the baseline, fonts define several metric lines:

```
                ┌─────────────────────────────┐  ascender (top of f, h, k, l)
                │  ┌──────────────────────────┤  cap-height (top of A, B, U)
                │  │  ┌──────────────────────┐│  x-height (top of a, e, m, n)
─── baseline ───┼──┼──┼──────────────────────┤│
                │  │  │  ┌───────────────────┘│  descender (bottom of g, p, q, y)
                └──┴──┴──┘                    │
```

Concrete proportions for Arial at 20pt (em = 20):

- **Cap-height**: ~73% of em → caps reach ~14.6pt above the baseline.
- **X-height**: ~52% of em → lowercase letters without ascenders reach ~10.4pt above.
- **Ascender**: ~73% of em → letters with ascenders (h, l, k) match cap-height roughly.
- **Descender**: ~21% of em → tails of g, p, q, y dip ~4.2pt below the baseline.

Punctuation sits in odd places:

- A **comma** drops just below the baseline (~1-2pt) and rises just above it (~1pt). Bbox is *below the line's center*.
- A **period** sits right on the baseline. Tiny bbox at baseline level.
- A **hyphen** floats near the middle of the x-height band, nowhere near the baseline.
- An **apostrophe** floats *high* — bottom around x-height, top around cap-height. Bbox is *above the line's center*, mirror image of a comma.
- A **colon** has dots above and below — its bbox spans most of the x-height band but the ink is concentrated at the two extremes.

So punctuation falls into two flavours: low-floaters (comma, period, descenders) and high-floaters (apostrophe, single quote, double quote). Any same-line rule has to accept both. Rules based on a single reference point (baseline, midpoint, top) tend to handle one flavour and fail on the other.

The takeaway: **glyphs on the same line share a baseline but almost nothing else** — not their tops, not their bottoms, not their midpoints, not their bbox heights. Any same-line grouping rule based on those quantities will fail on either descenders or punctuation or both.

## Bounding boxes (bbox)

Every text object in a PDF has a **bounding box** — the smallest rectangle containing its rendered glyphs:

```
top    ─── highest y the glyph reaches
left   right
bottom ─── lowest y the glyph reaches
```

Important: the bbox tracks the *rendered ink*, not the design metrics. So given the typography metrics above:

- Capital `U` on a 20pt line: top ≈ baseline + 14.6pt, bottom ≈ baseline.
- Lowercase `e`: top ≈ baseline + 10.4pt, bottom ≈ baseline.
- Descender `q`: top ≈ baseline + 10.4pt, bottom ≈ baseline − 4.2pt.
- Comma: top ≈ baseline + 1pt, bottom ≈ baseline − 1pt. Tiny.
- Hyphen: top and bottom both around baseline + 4-6pt.

All five share the same baseline but their bboxes are wildly different shapes. The handy consequence: for non-descender glyphs, **`bbox.bottom ≈ baseline`**. For descenders, `bbox.bottom < baseline`. So `max(bottom)` across a line's fragments is a robust estimate of the line's true baseline.

## Ink width vs advance width

Every glyph has two relevant horizontal measurements:

- **Advance width** — how far the cursor moves after drawing this glyph. The next glyph is positioned at `current_x + advance_width`. This is the layout property.
- **Ink width** — how wide the *visible drawn shape* actually is. Always less than or equal to the advance width.

The two diverge by varying amounts depending on the glyph:

```
N: |▓▓     ▓▓|        bbox.right = ink right
   ↑          ↑
   left       advance edge   ← ink ends here, but cursor moves to here

i:    |▓|              very thin ink
      ↑   ↑
      left advance
```

`N` is a wide letter where the diagonal stroke ends well before the right side of its advance box (~0.5pt of empty space inside the advance for 9pt Arial). `i` is a thin letter whose ink sits in the middle of an advance box that's mostly empty.

PDFium's `text_obj.bounds()` returns the **ink bbox**, not the advance box. So when measuring the gap between two adjacent glyphs:

```
ink_gap = next.bbox.left - prev.bbox.right
```

This is *not* the same as the gap between their advance edges, which is essentially zero for adjacent same-word glyphs. For `N` followed by `i` in 9pt Arial:

- Advance gap: ~0pt (the cursor moved straight to where `i` starts).
- Ink gap: ~1.4pt (empty space at the right of `N`'s advance + empty space at the left of `i`'s advance).

If your space-insertion threshold is 1.8pt for 9pt text, you're fine. If a different sans-serif font puts the gap at 2.0pt, you'd get `N igeria`.

The fix is to either pick a more generous threshold (we use `0.3 × em`, comfortably above any natural ink-gap-within-a-word and comfortably below a real space character which advances by ~`0.28 × em`) or to derive the threshold from font metrics directly. We took the cheap option.

## PDF coordinate system

PDFs use **Y-up**: `(0, 0)` is the bottom-left of the page, `top` is a *higher* number than `bottom`. This is the opposite of most screen/UI coordinate systems and easy to flip in your head.

So when iterating top-to-bottom of a page, you sort by `top` *descending*.

## How the bugs manifested

### Bug 1: per-glyph fragments tanked performance and broke alignment

Symptom: hundreds of one-letter fragments per page in `qemu_long_pdf.pdf`. Fix path needed two things:

1. **Pre-merge** adjacent fragments on a line that share style (`font_name`, `font_size`, bold, italic, underlined). Insert a space only when the horizontal gap is wider than ~20% of the font size — the empirical threshold between intra-word kerning and inter-word spacing.
2. **Move alignment to the line level**. The original code computed alignment per fragment by checking whether the fragment was horizontally centered on the page. A single 1-glyph fragment is never centered, so titles/subtitles/epigraphs were misclassified as left-aligned. Alignment now runs once per line against the line's combined `min(left)..max(right)` span.

### Bug 2: descenders and punctuation split off into ghost lines

Symptom: `USB Device Redirection for qemu-rdp` rendered as two blocks:

```
[Heading] USB De ice Redirection for d
[Heading] v qemu-r p
```

And paragraphs had floating commas:

```
... audio clipboard console keyboard , , , , , mouse multitouch ...
```

Cause: line-grouping used **midpoint distance** as the same-line test. For 20pt text:

- Cap glyph `U`: top=673.72, bottom=654.53, **midpoint=664.13**.
- Descender `q`: top=668.80, bottom=649.72, **midpoint=659.26**.
- Difference: 4.86pt. Threshold was 3pt. Split.

Same story for commas — their midpoints sit ~5pt below cap-height letters' midpoints because commas are short and hang near the baseline.

Once split, the descender glyphs (`v`, `q`, `p`, `d`) formed a phantom line under the heading; the commas formed phantom lines under the paragraphs. Sorting by `left` within those phantom lines and merging them into adjacent paragraph blocks produced the bunched commas at the right side of word groups.

First fix attempt: **vertical bbox overlap** as the same-line test instead of midpoint distance. Within one visual line, every glyph's bbox overlaps every other glyph's bbox — the line shares a baseline, and the bboxes share most of their vertical span — regardless of cap/x-height/descender/punctuation. Adjacent stacked lines, by contrast, have *negative* overlap (the inter-line gap).

```rust
let overlap = current_top.min(frag.top) - current_bottom.max(frag.bottom);
let on_same_line = overlap > 0.5;  // small cushion for float slop
```

This fixed the qemu PDFs cleanly. But it broke later when we threw a multi-column resume at it (see Bug 3). The bbox-overlap rule was eventually replaced.

### Bug 3: multi-column layouts collapsed into garbled lines

Symptom (resume PDF): the two-column header — a giant "Abass Azeez Afolarin" on the left, three small `+234... | email | github | linkedin | thefola.dev` lines on the right — came out as:

```
Abass Azeez Afolarin github.com/azeezaba+s2s324 0095 1 6 |
l0in6k4 e9d1in2.c4 o| maz/eine/zaabbaassss-a2z0e0e5z@-5gam95a5il4.c2o5m9
Full Stack Software Engineer thefola.dev | Nigeria...
```

Per-glyph fragments from three small right-column lines were being interleaved into the big-header line by their `left` positions.

Cause: the bbox-overlap rule from Bug 2 is *too* tolerant when font sizes differ. A 20pt header glyph has a bbox ~17pt tall; a 9pt right-column glyph has a bbox ~7pt tall. The 20pt header bbox vertically overlaps three stacked 9pt lines on the same horizontal band of the page, so the grouper merges all four into one "line" and sorts the resulting stew by x-position.

Fix: **same-line = (baseline match) AND (font sizes compatible)**.

```rust
let baseline_diff = (current_baseline - frag.bottom).abs();
let baseline_compatible = baseline_diff < min_fs * 0.4;
let font_compatible = max_fs / min_fs < 1.5;
let on_same_line = baseline_compatible && font_compatible;
```

- `current_baseline` is tracked as `max(bottom)` across the line's fragments — a robust baseline estimate (see typography section above).
- `baseline_diff < min_fs * 0.4` accommodates descenders (~21% of em below baseline) and commas (~1pt below baseline) while still separating adjacent stacked lines.
- The font-size ratio cap (1.5) is the new ingredient. It refuses to group a 9pt fragment with a 20pt line even when their baselines accidentally align — which happens a lot in two-column layouts where the small right-column text is vertically centered against the big left-column header.

### Bug 4 ½: cluster fragmentation in interleaved iteration

Symptom (resume PDF, after Bug 3's fix): the name `Abass Azeez Afolarin` came out as `Ab A Af l i . ass zeez o ar n` — caps and ascender letters split off from x-height letters of the *same word*. Commas regressed to bunching at the end of paragraphs (`audio clipboard console keyboard , , , , ,`). And `Nigeria` showed up as `N igeria`.

The first two share a root cause that's not about the same-line *test* — it's about the same-line *bookkeeping*.

After sorting fragments by `top` descending, the iteration order interleaves glyphs from different visual lines whose top ranges overlap:

- `top = 754` — header caps (`A`, `b`, `f`, `l`, `i`)
- `top = 752` — right-column caps (`+`, `2`, `3`, `4`)
- `top = 750` — header x-height (`a`, `s`, `s`, `z`, `e`, ...) **and** right-column x-height letters
- `top = 745` — right-column descender bottoms

The previous code tracked **one active line** at a time. Whenever a fragment arrived whose baseline didn't match the active line, it closed that line and started a new one — even when the fragment belonged to a *different* line that was active two iterations ago. So as the loop walked through the interleaved sort order, it kept ping-ponging between header and right-column, fragmenting both into many tiny sub-lines that downstream paragraph merging then stitched together in the wrong order.

Fix: **cluster-based grouping**. Each fragment scans the entire active set of clusters and joins the first one it matches:

```rust
match clusters.iter().position(|c| c.accepts(&frag)) {
    Some(idx) => clusters[idx].absorb(frag),
    None => clusters.push(LineCluster::new(frag)),
}
```

Same-line fragments encountered in any iteration order accrete onto their proper cluster. The acceptance test is the same baseline + font-ratio rule from Bug 3, just applied against every active cluster instead of only the most-recent one.

While reworking this, two related thresholds got loosened:

- **Baseline tolerance** went from `0.4 × min_font_size` to `0.5 × min_font_size`. The 0.4 cutoff was borderline for commas in some sans-serif fonts and for cap-vs-x-height bbox-bottom variation. 0.5 covers them comfortably while still rejecting adjacent stacked lines (whose baselines differ by ~1.2 × em).
- **Space-insertion threshold** went from `0.2 × font_size` to `0.3 × font_size`. The bbox-right of `N` in many sans-serif fonts ends well before its advance width, so the ink gap to the following `i` can exceed 1.8pt at 9pt — enough to trigger the old threshold and produce `N igeria`. 0.3 × em is wide enough to absorb that ink-vs-advance discrepancy and narrow enough to still flag a real space character as a real space.

### Bug 4 ¾: apostrophes (and other above-baseline punctuation) split off

Symptom: `I'm` and `I've` came out as `I m` and `I ve`, with the apostrophes piled up later in the paragraph as `' '`.

Same shape as the original comma bug, but flipped vertically. Apostrophes sit *above* the baseline:

- Apostrophe top ≈ baseline + 0.7 × em
- Apostrophe bottom ≈ baseline + 0.5 × em (around x-height)

In 9pt body text, the apostrophe's `bottom` is roughly 4.5pt above the line's baseline. The Round 4 baseline rule was:

```rust
baseline_diff < min_fs * 0.5
// 4.5 < 9 * 0.5 = 4.5  → strict less-than fails
```

Right at the threshold and rejected. Worse: when the apostrophe arrived first (its top is similar to caps' tops, so sort-by-top-desc could place it before regular letters), it seeded a cluster with `baseline = apostrophe.bottom`. Subsequent letters had `bottom` 4.5pt below that polluted baseline and couldn't join.

Bumping the tolerance to 0.7 × em would have fixed apostrophes but reintroduced the "letter pollutes baseline" failure mode and weakened the multi-column separation.

The real fix was to drop the baseline test and go back to **bbox vertical overlap** (the Round 2 rule), keeping Round 3's font-size-ratio cap. Bbox overlap is symmetric — it doesn't care whether punctuation sits above the baseline or below, only that the punctuation's band overlaps the line's combined band. Apostrophes overlap by ~1.8pt, descenders by ~10pt, commas by ~1pt — all comfortably above the 0.5pt minimum. Adjacent stacked lines still produce *negative* overlap (the inter-line gap), so they don't join.

The font-ratio cap is what makes this safe in multi-column layouts. Round 2's pure bbox-overlap rule failed there because a tall 20pt bbox vertically overlaps a stack of 9pt bboxes; combining bbox overlap with `max_font_size / min_font_size < 1.5` rejects those pairings while accepting punctuation-vs-letter pairings (which always have ratio = 1).

```rust
fn accepts(&self, frag: &TextFragment) -> bool {
    let overlap = self.top.min(frag.top) - self.bottom.max(frag.bottom);
    let min_fs = self.max_font_size.min(frag.font_size).max(0.1);
    let max_fs = self.max_font_size.max(frag.font_size);
    overlap > 0.5 && max_fs / min_fs < 1.5
}
```

The `LineCluster` no longer needs to track a `baseline` field — only `top`, `bottom`, and `max_font_size`.

The bigger lesson: when a same-line test treats above-baseline and below-baseline glyphs asymmetrically (anything based on `frag.bottom` does), it'll fail on whichever side you didn't think about. Bbox overlap is the test that doesn't care.

### Bug 4: full-width column rows mistaken for centered titles

Symptom: in the same resume, every job entry's "Title… …Date" row got classified as `[Subtitle]`, which the classifier reserves for centered, small, non-bold lines.

Cause: the line spans from the left margin (~70pt) to the right margin (~540pt) on a 612pt-wide page. The center-detection formula `|((page_width - line_width) / 2) - leftmost| ≤ 3` gets fooled — the line's outer edges are symmetric about the page center, even though the *content* is hugging both edges with empty space in between.

Fix: a centered line is contiguous; reject `Center` when there's a big horizontal gap between adjacent fragments inside the line.

```rust
let max_internal_gap = fragments.windows(2)
    .map(|pair| pair[1].left - pair[0].right)
    .fold(0.0_f32, f32::max);
let has_column_gap = max_internal_gap > page_width * 0.1;

if left_diff <= 3.0 && !has_column_gap {
    TextAlign::Center
} else {
    TextAlign::Left
}
```

The 10%-of-page-width threshold is comfortably above any natural inter-word space (a wide space at 9pt is ~3pt, and at 20pt is ~6pt) and comfortably below any genuine column gap.

## Why the Ctrl+P pipeline accidentally fixed everything

Ctrl+P → "Save as PDF" doesn't preserve the source PDF's structure. It rasterizes (or re-flows) the page through the OS print pipeline and writes a fresh PDF. That fresh PDF uses:

- Full (non-subsetted) fonts — no `XXXXXX+` prefix.
- `Tj` runs per word — no per-glyph `TJ` arrays.
- Bbox-friendly text objects with sensible advance widths.

So `qemu_pdf.pdf` looked easy to parse not because it *was* easy, but because the print pipeline had already done the per-glyph-to-words consolidation we now do ourselves in `merge_line_fragments`.

## Takeaways

- Don't trust per-fragment metadata in a per-glyph PDF — promote line-level decisions to the line level.
- Same-line grouping should cluster by **bbox vertical overlap**, with a font-size ratio cap. Bbox overlap is the only test that's symmetric across both flavours of punctuation: it doesn't care whether a glyph sits above the baseline (apostrophe, quote) or below (comma, period, descender). Any rule based on a single reference point (baseline, midpoint, top) handles one flavour and fails on the other.
- The font-size ratio cap (~1.5) is what makes bbox overlap viable in multi-column layouts. Without it, a tall big-font bbox on one side of the page absorbs several small-font lines whose bboxes vertically overlap it.
- Group fragments using **cluster-based assignment**, not a single rolling active-line. With multiple columns you'll see fragments from several lines interleaved in any reasonable sort order, and a single-active-line loop keeps closing and reopening the wrong line.
- "Centered alignment" by symmetric outer margins is a false positive on multi-column rows. Add a no-big-internal-gap check to distinguish centered text from `Title……Date`-style spans.
- Bbox right is *ink* right, not *advance* right. Ink-gap between same-word glyphs can be wider than you'd expect, especially for letters like `N` followed by narrow letters. Pick your space-insertion threshold accordingly (`~0.3 × em` is a robust default).
- The `XXXXXX+` font-name prefix is your early warning that you're about to deal with a per-glyph stream.
- When something works on a "simplified" version of an input but not the real one, suspect that the simplification path is doing work you'll need to do yourself.
