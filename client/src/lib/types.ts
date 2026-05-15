// Mirrors the Rust types in server/src/pdf_inference/reconstruct.rs.
// Serde defaults to externally-tagged enums, so each ContentBlock arrives
// as either { "Text": {...} } or { "Table": {...} }.

export type BlockKind =
  | 'PageNumber'
  | 'Title'
  | 'Subtitle'
  | 'Epigraph'
  | 'Attribution'
  | 'Paragraph'
  | 'Heading'
  | 'ListItem'
  | 'SubListItem'
  | 'TableOfContentsHeading';

export interface TextBlock {
  kind: BlockKind;
  text: string;
  font_size: number;
  is_bold: boolean;
  is_italic: boolean;
  is_underlined: boolean;
}

export interface TableCell {
  text: string;
  is_bold: boolean;
  is_italic: boolean;
  col_span: number;
  row_span: number;
}

export interface TableRow {
  cells: TableCell[];
}

export interface TableBlock {
  rows: TableRow[];
}

export type ContentBlock = { Text: TextBlock } | { Table: TableBlock };

export type SemanticPage = ContentBlock[];

export interface SemanticData {
  pdf_semantic_data: SemanticPage[];
  file_name?: string;
  message?: string;
}

export function isTextBlock(block: ContentBlock): block is { Text: TextBlock } {
  return 'Text' in block;
}

export function isTableBlock(block: ContentBlock): block is { Table: TableBlock } {
  return 'Table' in block;
}

// Searchable text for any block. Table cells are joined with spaces so a
// search query can match across a row.
export function blockSearchText(block: ContentBlock): string {
  if (isTextBlock(block)) return block.Text.text;
  return block.Table.rows
    .flatMap((row) => row.cells.map((cell) => cell.text))
    .join(' ');
}

// Coarse label used in the search-result chip.
export function blockSearchKind(block: ContentBlock): string {
  if (isTextBlock(block)) return block.Text.kind;
  return 'Table';
}
