// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use polars::prelude::*;
use serde::Deserialize;

// ─── Column sanitisation ────────────────────────────────────────────────────

/// Replace any character that isn't alphanumeric or `_` with `_` so polars
/// SQL can select every column without quoting tricks.
fn sanitize_column_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn sanitize_lf(lf: LazyFrame) -> Result<LazyFrame, PolarsError> {
    let schema = lf.schema()?;
    let (old_names, new_names): (Vec<String>, Vec<String>) = schema
        .iter_names()
        .filter(|n| n.chars().any(|c| !c.is_alphanumeric() && c != '_'))
        .map(|n| (n.to_string(), sanitize_column_name(n.as_str())))
        .unzip();
    if old_names.is_empty() {
        return Ok(lf);
    }
    Ok(lf.rename(old_names, new_names))
}

// ─── JSON / JSONL support ────────────────────────────────────────────────────

type JsonRow = serde_json::Map<String, serde_json::Value>;

/// Infer type and build a polars Series from a slice of JSON values.
fn json_column_to_series(name: &str, values: &[serde_json::Value]) -> Series {
    // Bool
    if values.iter().all(|v| v.is_null() || v.is_boolean()) {
        let mut b = BooleanChunkedBuilder::new(name.into(), values.len());
        for v in values {
            if v.is_null() {
                b.append_null();
            } else {
                b.append_value(v.as_bool().unwrap());
            }
        }
        return b.finish().into_series();
    }
    // i64
    if values.iter().all(|v| v.is_null() || v.is_i64()) {
        let mut b = PrimitiveChunkedBuilder::<Int64Type>::new(name.into(), values.len());
        for v in values {
            if v.is_null() {
                b.append_null();
            } else {
                b.append_value(v.as_i64().unwrap());
            }
        }
        return b.finish().into_series();
    }
    // f64
    if values.iter().all(|v| v.is_null() || v.is_number()) {
        let mut b = PrimitiveChunkedBuilder::<Float64Type>::new(name.into(), values.len());
        for v in values {
            if v.is_null() {
                b.append_null();
            } else {
                b.append_value(v.as_f64().unwrap());
            }
        }
        return b.finish().into_series();
    }
    // String fallback — arrays/objects are JSON-serialised
    let mut b = StringChunkedBuilder::new(name.into(), values.len());
    for v in values {
        if v.is_null() {
            b.append_null();
        } else if let Some(s) = v.as_str() {
            b.append_value(s);
        } else {
            b.append_value(&v.to_string());
        }
    }
    b.finish().into_series()
}

fn objects_to_df(rows: Vec<JsonRow>) -> Result<DataFrame, PolarsError> {
    if rows.is_empty() {
        return Ok(DataFrame::empty());
    }
    let keys: Vec<String> = rows[0].keys().cloned().collect();
    let series: Vec<Series> = keys
        .iter()
        .map(|key| {
            let vals: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| r.get(key).cloned().unwrap_or(serde_json::Value::Null))
                .collect();
            json_column_to_series(key, &vals)
        })
        .collect();
    DataFrame::new(series)
}

/// Read one or more JSON / JSONL files into a list of row-objects.
///
/// JSON supports:
///   - array-of-objects  `[{...}, {...}]`
///   - object-of-arrays  `{"col": [v, v, ...]}`
///
/// JSONL: one JSON object per line.
///   Extra keys absent from some objects are discarded; a human-readable
///   warning is returned so the caller can surface it to the user.
fn read_json_objects(
    paths: &[PathBuf],
    filetype: &str,
) -> Result<(Vec<JsonRow>, Option<String>), PolarsError> {
    let ce = |msg: String| PolarsError::ComputeError(msg.into());
    let mut rows: Vec<JsonRow> = Vec::new();

    for path in paths {
        let text = std::fs::read_to_string(path).map_err(|e| ce(e.to_string()))?;
        if filetype == "jsonl" {
            for (i, line) in text
                .lines()
                .enumerate()
                .filter(|(_, l)| !l.trim().is_empty())
            {
                let obj: JsonRow =
                    serde_json::from_str(line).map_err(|e| ce(format!("line {}: {}", i + 1, e)))?;
                rows.push(obj);
            }
        } else {
            let val: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| ce(e.to_string()))?;
            match val {
                serde_json::Value::Array(arr) => {
                    for item in arr {
                        match item {
                            serde_json::Value::Object(obj) => rows.push(obj),
                            _ => return Err(ce("JSON array items must be objects".into())),
                        }
                    }
                }
                serde_json::Value::Object(col_map) => {
                    // {"col": [v0, v1, ...], ...}
                    let n = col_map
                        .values()
                        .next()
                        .and_then(|v| v.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0);
                    for i in 0..n {
                        let mut row = JsonRow::new();
                        for (k, arr) in &col_map {
                            let v = arr
                                .as_array()
                                .and_then(|a| a.get(i))
                                .cloned()
                                .unwrap_or(serde_json::Value::Null);
                            row.insert(k.clone(), v);
                        }
                        rows.push(row);
                    }
                }
                _ => {
                    return Err(ce(
                        "JSON must be an array-of-objects or an object-of-arrays".into(),
                    ))
                }
            }
        }
    }

    // JSONL: intersect keys and warn about any that were dropped
    let warning = if filetype == "jsonl" && rows.len() > 1 {
        let all_keys: HashSet<String> = rows.iter().flat_map(|r| r.keys().cloned()).collect();
        let common: HashSet<String> = rows
            .iter()
            .map(|r| r.keys().cloned().collect::<HashSet<_>>())
            .reduce(|a, b| a.intersection(&b).cloned().collect())
            .unwrap_or_default();
        let n_disc = all_keys.len().saturating_sub(common.len());
        if n_disc > 0 {
            let mut disc: Vec<String> = all_keys.difference(&common).cloned().collect();
            disc.sort();
            for row in &mut rows {
                row.retain(|k, _| common.contains(k));
            }
            Some(format!(
                "{n_disc} key(s) discarded (not present in every object): {}",
                disc.join(", ")
            ))
        } else {
            None
        }
    } else {
        None
    };

    Ok((rows, warning))
}

// ─── File entries (multi-table context) ─────────────────────────────────────

#[derive(Deserialize)]
struct FileEntry {
    alias: String,
    filenames: Vec<String>,
    filetype: String,
    sep: Option<u8>, // CSV separator as byte value
}

fn entry_to_lf(entry: &FileEntry) -> Result<LazyFrame, PolarsError> {
    let paths: Arc<[PathBuf]> = entry.filenames.iter().map(PathBuf::from).collect();
    let lf = match entry.filetype.as_str() {
        "parquet" => LazyFrame::scan_parquet_files(paths, Default::default())?,
        "arrow" => LazyFrame::scan_ipc_files(paths, Default::default())?,
        "csv" => LazyCsvReader::new_paths(paths)
            .with_missing_is_null(true)
            .with_separator(entry.sep.unwrap_or(b','))
            .finish()?,
        "json" | "jsonl" => {
            let path_bufs: Vec<PathBuf> = entry.filenames.iter().map(PathBuf::from).collect();
            let (rows, _) = read_json_objects(&path_bufs, entry.filetype.as_str())?;
            return sanitize_lf(objects_to_df(rows)?.lazy());
        }
        other => {
            return Err(PolarsError::InvalidOperation(
                format!("unknown filetype: {other}").into(),
            ))
        }
    };
    sanitize_lf(lf)
}

fn build_context(entries: &[FileEntry]) -> Result<polars::sql::SQLContext, PolarsError> {
    let mut ctx = polars::sql::SQLContext::new();
    for entry in entries {
        ctx.register(&entry.alias, entry_to_lf(entry)?);
    }
    Ok(ctx)
}

fn parse_entries(entries_json: &str) -> Result<Vec<FileEntry>, String> {
    serde_json::from_str(entries_json)
        .map_err(|e| serde_json::json!({ "err_msg": e.to_string() }).to_string())
}

// ─── Output helpers ──────────────────────────────────────────────────────────

/// Recursively convert an AnyValue to a serde_json::Value.
/// Used so that List columns are emitted as proper JSON arrays instead of
/// Polars' Display representation, which truncates long lists with `…`.
fn anyvalue_to_json(val: AnyValue<'_>) -> serde_json::Value {
    match val {
        AnyValue::Null => serde_json::Value::Null,
        AnyValue::Boolean(b) => serde_json::json!(b),
        AnyValue::Int8(i) => serde_json::json!(i),
        AnyValue::Int16(i) => serde_json::json!(i),
        AnyValue::Int32(i) => serde_json::json!(i),
        AnyValue::Int64(i) => serde_json::json!(i),
        AnyValue::UInt8(u) => serde_json::json!(u),
        AnyValue::UInt16(u) => serde_json::json!(u),
        AnyValue::UInt32(u) => serde_json::json!(u),
        AnyValue::UInt64(u) => serde_json::json!(u),
        AnyValue::Float32(f) => serde_json::json!(f),
        AnyValue::Float64(f) => serde_json::json!(f),
        AnyValue::String(s) => serde_json::json!(s),
        AnyValue::List(s) => serde_json::Value::Array(s.iter().map(anyvalue_to_json).collect()),
        other => serde_json::json!(other.to_string()),
    }
}

/// Produce a display string for a single cell value.
/// List columns become compact JSON arrays (no Polars truncation).
fn anyvalue_to_display(val: AnyValue<'_>) -> String {
    match &val {
        AnyValue::List(_) => {
            let json_val = anyvalue_to_json(val);
            serde_json::to_string(&json_val).unwrap_or_default()
        }
        AnyValue::Null => String::new(),
        other => other.to_string(),
    }
}

fn generate_table(df: &DataFrame) -> String {
    let col_names = df.get_column_names();
    let col_types = df.dtypes();
    let row_count = df.height();

    let headers = col_names
        .iter()
        .zip(col_types.iter())
        .map(|(k, _v)| {
            serde_json::json!({
                "title": k.to_string(),
                "key":   k.to_string(),
                "resizable": true,
            })
        })
        .collect::<Vec<_>>();

    let mut iters = df.iter().map(|s| s.iter()).collect::<Vec<_>>();
    let body = (0..row_count)
        .map(|_| {
            iters
                .iter_mut()
                .zip(col_names.iter())
                .map(|(it, name)| (name.to_string(), anyvalue_to_display(it.next().unwrap())))
                .collect::<HashMap<_, _>>()
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&serde_json::json!({
        "col_count": col_names.len(),
        "row_count": row_count,
        "headers":   headers,
        "body":      body,
    }))
    .unwrap()
}

fn deal_error(e: PolarsError) -> String {
    serde_json::json!({ "err_msg": e.to_string() }).to_string()
}

fn io_error(e: std::io::Error) -> String {
    serde_json::json!({ "err_msg": e.to_string() }).to_string()
}

// ─── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
fn execute_query(entries_json: &str, sql: &str) -> String {
    let entries = match parse_entries(entries_json) {
        Ok(e) => e,
        Err(s) => return s,
    };
    let result = build_context(&entries)
        .and_then(|mut ctx| ctx.execute(sql))
        .and_then(|lf| lf.collect())
        .and_then(|df| df.with_row_index("idx", Some(1)));
    match result {
        Ok(df) => generate_table(&df),
        Err(e) => deal_error(e),
    }
}

#[tauri::command]
fn export_parquet(entries_json: &str, sql: &str, dest: &str) -> String {
    let entries = match parse_entries(entries_json) {
        Ok(e) => e,
        Err(s) => return s,
    };
    // Run query first; only create the file if the query succeeds.
    let mut df = match build_context(&entries)
        .and_then(|mut ctx| ctx.execute(sql))
        .and_then(|lf| lf.collect())
    {
        Ok(df) => df,
        Err(e) => return deal_error(e),
    };
    let file = match std::fs::File::create(dest) {
        Ok(f) => f,
        Err(e) => return io_error(e),
    };
    match ParquetWriter::new(file).finish(&mut df) {
        Ok(_) => serde_json::json!({ "ok": true }).to_string(),
        Err(e) => deal_error(e),
    }
}

#[tauri::command]
fn export_csv_file(entries_json: &str, sql: &str, dest: &str, sep: u8) -> String {
    let entries = match parse_entries(entries_json) {
        Ok(e) => e,
        Err(s) => return s,
    };
    let mut df = match build_context(&entries)
        .and_then(|mut ctx| ctx.execute(sql))
        .and_then(|lf| lf.collect())
    {
        Ok(df) => df,
        Err(e) => return deal_error(e),
    };
    let file = match std::fs::File::create(dest) {
        Ok(f) => f,
        Err(e) => return io_error(e),
    };
    match CsvWriter::new(file).with_separator(sep).finish(&mut df) {
        Ok(_) => serde_json::json!({ "ok": true }).to_string(),
        Err(e) => deal_error(e),
    }
}

#[tauri::command]
fn get_schema(entries_json: &str) -> String {
    let entries = match parse_entries(entries_json) {
        Ok(e) => e,
        Err(s) => return s,
    };
    let mut tables = Vec::new();
    for entry in &entries {
        let lf = match entry_to_lf(entry) {
            Ok(lf) => lf,
            Err(e) => return deal_error(e),
        };
        let schema = match lf.schema() {
            Ok(s) => s,
            Err(e) => return deal_error(e),
        };
        let columns: Vec<serde_json::Value> = schema
            .iter()
            .map(|(name, dtype)| {
                serde_json::json!({
                    "name":  name.to_string(),
                    "dtype": dtype.to_string(),
                })
            })
            .collect();
        tables.push(serde_json::json!({
            "alias":   entry.alias.clone(),
            "columns": columns,
        }));
    }
    serde_json::json!({ "ok": true, "tables": tables }).to_string()
}

#[tauri::command]
fn write_text_file(content: &str, dest: &str) -> String {
    match std::fs::write(dest, content) {
        Ok(_) => serde_json::json!({ "ok": true }).to_string(),
        Err(e) => io_error(e),
    }
}

#[tauri::command]
fn check_json_import(paths: Vec<String>, filetype: String) -> String {
    let path_bufs: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    match read_json_objects(&path_bufs, &filetype) {
        Ok((_, warning)) => serde_json::json!({ "ok": true, "warning": warning }).to_string(),
        Err(e) => deal_error(e),
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            execute_query,
            export_parquet,
            export_csv_file,
            write_text_file,
            get_schema,
            check_json_import,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
