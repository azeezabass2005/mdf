// Mirrors the Rust types in server/src/pdf_inference/reconstruct.rs.
// Serde defaults to externally-tagged enums, so each ContentBlock arrives
// as { "Text": {...} }, { "Table": {...} }, or { "Image": {...} }.

export type BlockKind =
  | 'PageNumber'
  | 'Heading1'
  | 'Heading2'
  | 'Heading3'
  | 'Heading4'
  | 'Heading5'
  | 'Heading6'
  | 'HeadingCandidate'
  | 'Epigraph'
  | 'Attribution'
  | 'Paragraph'
  | 'ListItem'
  | 'SubListItem'
  | 'OrderedListItem'
  | 'TableOfContentsHeading';

export interface TextBlock {
  kind: BlockKind;
  text: string;
  font_size: number;
  is_bold: boolean;
  is_italic: boolean;
  is_underlined: boolean;
  y_position: number;
}

export interface TableCell {
  text: string;
  is_bold: boolean;
  is_italic: boolean;
  is_underlined: boolean;
  col_span: number;
  row_span: number;
}

export interface TableRow {
  cells: TableCell[];
}

export interface TableBlock {
  rows: TableRow[];
  col_count: number;
  y_position: number;
}

export interface ImageBlock {
  data_uri: string;
  width_pt: number;
  height_pt: number;
  y_position: number;
}

export type ContentBlock =
  | { Text: TextBlock }
  | { Table: TableBlock }
  | { Image: ImageBlock };

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

export function isImageBlock(block: ContentBlock): block is { Image: ImageBlock } {
  return 'Image' in block;
}

// Searchable text for any block. Table cells are joined with spaces so a
// search query can match across a row; images contribute nothing.
export function blockSearchText(block: ContentBlock): string {
  if (isTextBlock(block)) return block.Text.text;
  if (isTableBlock(block)) {
    return block.Table.rows.flatMap((row) => row.cells.map((cell) => cell.text)).join(' ');
  }
  return '';
}

// Coarse label used in the search-result chip.
export function blockSearchKind(block: ContentBlock): string {
  if (isTextBlock(block)) return block.Text.kind;
  if (isTableBlock(block)) return 'Table';
  return 'Image';
}
