export interface FileEntry {
    id: string
    alias: string
    filenames: string[]
    filetype: 'parquet' | 'arrow' | 'csv' | 'json' | 'jsonl'
    sep: string
}

export interface ColumnInfo {
    name: string
    dtype: string
}

export type SearchMode = 'text' | 'fuzzy' | 'regex'
