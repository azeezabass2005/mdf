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

Work in progress.