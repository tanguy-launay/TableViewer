export interface FileEntry {
  id: string;
  alias: string;
  filenames: string[];
  filetype:
    | "parquet"
    | "arrow"
    | "csv"
    | "json"
    | "jsonl"
    | "yaml"
    | "xml"
    | "toml"
    | "duckdb"
    | "sqlite";
  sep: string;
  jaq_filter?: string; // for json/jsonl/yaml/xml/toml — empty/undefined = identity
}

export interface ColumnInfo {
  name: string;
  dtype: string;
}

export type SearchMode = "text" | "fuzzy" | "regex";
