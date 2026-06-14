# MDF - Maldives for PDFs

Most PDFs are unpleasant to read. Bad fonts, tight spacing, no control over how the content looks. MDF fixes that by converting your PDFs into clean, readable web pages you can actually enjoy reading.

## What it does

- Upload a PDF and read it as a web page
- Choose between scroll mode or slide-per-page mode
- Pick a theme and font built for readability
- Accessibility support included

## Stack

- **Backend** - Rust (Axum) - handles PDF ingestion and content extraction
- **Frontend** - Svelte - handles rendering, themes, and the reading experience

## Project Structure

```
mdf/
├── server/   # Rust backend
└── client/   # Svelte frontend
```

## Getting Started

### Backend

```bash
cd server
cargo build --bins
./target/debug/pdf-maldives
```

### Frontend

```bash
cd client
bun install
bun run dev
```

## Status

Experimental.

PDFs are a mess. The format lets you draw anything anywhere, so two documents that look identical on screen can have wildly different internal structures. That makes a universal parser more of a research problem than a weekend project, and I'm not pretending to have solved it.

Here's what currently works reasonably well:

- Text-heavy documents with a single column
- Headings, paragraphs, bold and italic
- Tables that are drawn with visible borders
- Diagrams (rendered as images so the content stays intact)

Here's what doesn't work well, or at all:

- Scanned PDFs (no OCR yet)
- Multi-column layouts get messy
- Tables without ruled lines are ignored
- Math, equations, and footnotes are not handled specially
- Some fonts confuse the extractor and produce garbled spacing

I might come back to it. For now I'm calling it done as a v0.1 and moving on to other things.