// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use duckdb::types::Value;
use serde::Deserialize;

// ─── Unique temp-file naming ─────────────────────────────────────────────────

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

fn next_temp_path() -> PathBuf {
    let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("tv_{pid}_{id}.ndjson"))
}

// ─── RAII temp-file guard ────────────────────────────────────────────────────

struct TempFile(PathBuf);
impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

// ─── File-entry descriptor ───────────────────────────────────────────────────

#[derive(Deserialize)]
struct FileEntry {
    alias: String,
    filenames: Vec<String>,
    filetype: String,
    sep: Option<u8>,
    jaq_filter: Option<String>,
}

// ─── DuckDB Value → serde_json::Value ────────────────────────────────────────
// Value has 26 variants; we serialise complex ones to JSON, scalars directly.

fn value_to_json(val: &Value) -> serde_json::Value {
    match val {
        Value::Null => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::json!(b),
        Value::TinyInt(i) => serde_json::json!(i),
        Value::SmallInt(i) => serde_json::json!(i),
        Value::Int(i) => serde_json::json!(i),
        Value::BigInt(i) => serde_json::json!(i),
        Value::HugeInt(i) => serde_json::json!(i.to_string()),
        Value::UTinyInt(u) => serde_json::json!(u),
        Value::USmallInt(u) => serde_json::json!(u),
        Value::UInt(u) => serde_json::json!(u),
        Value::UBigInt(u) => serde_json::json!(u),
        Value::Float(f) => serde_json::json!(f),
        Value::Double(f) => serde_json::json!(f),
        Value::Decimal(d) => serde_json::json!(d.to_string()),
        Value::Text(s) | Value::Enum(s) => serde_json::Value::String(s.clone()),
        Value::Blob(b) => serde_json::Value::String(format!("<blob {} B>", b.len())),
        Value::Date32(d) => serde_json::json!(d),
        Value::Timestamp(_, v) | Value::Time64(_, v) => serde_json::json!(v),
        Value::Interval {
            months,
            days,
            nanos,
        } => serde_json::Value::String(format!("{months}m {days}d {nanos}ns")),
        Value::List(items) | Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(value_to_json).collect())
        }
        Value::Struct(s) => {
            let obj: serde_json::Map<_, _> = s
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        Value::Map(m) => {
            let obj: serde_json::Map<_, _> = m
                .iter()
                .map(|(k, v)| (format!("{k:?}"), value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        Value::Union(inner) => value_to_json(inner),
    }
}

fn value_to_display(val: &Value) -> String {
    match val {
        Value::Null => String::new(),
        Value::Boolean(b) => b.to_string(),
        Value::TinyInt(i) => i.to_string(),
        Value::SmallInt(i) => i.to_string(),
        Value::Int(i) => i.to_string(),
        Value::BigInt(i) => i.to_string(),
        Value::HugeInt(i) => i.to_string(),
        Value::UTinyInt(u) => u.to_string(),
        Value::USmallInt(u) => u.to_string(),
        Value::UInt(u) => u.to_string(),
        Value::UBigInt(u) => u.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Double(f) => f.to_string(),
        Value::Decimal(d) => d.to_string(),
        Value::Text(s) | Value::Enum(s) => s.clone(),
        Value::Blob(b) => format!("<blob {} B>", b.len()),
        Value::Date32(d) => d.to_string(),
        Value::Timestamp(_, v) | Value::Time64(_, v) => v.to_string(),
        Value::Interval {
            months,
            days,
            nanos,
        } => format!("{months}m {days}d {nanos}ns"),
        // Complex → compact JSON so the cell-detail modal can pretty-print them
        Value::List(_) | Value::Array(_) | Value::Struct(_) | Value::Map(_) | Value::Union(_) => {
            serde_json::to_string(&value_to_json(val)).unwrap_or_default()
        }
    }
}

// ─── Error helper ────────────────────────────────────────────────────────────

fn err_json(msg: &str) -> String {
    serde_json::json!({"err_msg": msg}).to_string()
}

// ─── jaq preprocessing helpers ──────────────────────────────────────────────

fn file_to_json_value(path: &Path, filetype: &str) -> Result<serde_json::Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    match filetype {
        "json" => serde_json::from_str(&text).map_err(|e| e.to_string()),
        "jsonl" | "ndjson" => {
            let rows: Result<Vec<serde_json::Value>, _> = text
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(serde_json::from_str)
                .collect();
            rows.map(serde_json::Value::Array)
                .map_err(|e| e.to_string())
        }
        "yaml" | "yml" => serde_yaml::from_str(&text).map_err(|e| e.to_string()),
        "toml" => {
            let v: toml::Value = toml::from_str(&text).map_err(|e| e.to_string())?;
            serde_json::to_value(v).map_err(|e| e.to_string())
        }
        "xml" => xml_to_json_value(&text),
        other => Err(format!("unsupported format: {other}")),
    }
}

/// Apply a jq-compatible filter using jaq.  Returns all output values.
fn apply_jaq(
    filter_str: &str,
    input: &serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    use jaq_all::{data, fmts, jaq_core::Vars};

    let filter = data::compile(filter_str).map_err(|e| format!("jaq compile: {e:?}"))?;

    // Serialize to JSON string then parse into a jaq Val
    let input_str = serde_json::to_string(input).map_err(|e| e.to_string())?;
    let input_val = fmts::read::json::parse_single(input_str.as_bytes())
        .map_err(|e| format!("jaq parse: {e}"))?;

    let runner = data::Runner::default();
    let vars = Vars::new([]);
    let mut out: Vec<serde_json::Value> = Vec::new();

    // data::run is callback-based (cannot return an iterator due to RcIter)
    data::run(
        &runner,
        &filter,
        vars,
        std::iter::once(Ok::<_, String>(input_val)),
        |e| e,
        |val_x| -> Result<(), String> {
            match val_x {
                // Val implements Display → compact JSON string
                Ok(val) => {
                    let s = format!("{val}");
                    serde_json::from_str::<serde_json::Value>(&s)
                        .map(|v| out.push(v))
                        .map_err(|e| format!("jaq output: {e}"))
                }
                Err(e) => Err(format!("jaq runtime: {e:?}")),
            }
        },
    )
    .map_err(|e| e)?;

    Ok(out)
}

/// Stack-based XML → JSON using quick-xml events.
fn xml_to_json_value(xml: &str) -> Result<serde_json::Value, String> {
    use quick_xml::{events::Event, Reader};
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut stack: Vec<(String, serde_json::Map<String, serde_json::Value>)> = vec![];
    let mut root = serde_json::Value::Null;
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let mut obj = serde_json::Map::new();
                for attr in e.attributes().flatten() {
                    let k = format!("@{}", String::from_utf8_lossy(attr.key.as_ref()));
                    let v = String::from_utf8_lossy(&attr.value).into_owned();
                    obj.insert(k, serde_json::Value::String(v));
                }
                stack.push((tag, obj));
            }
            Event::Text(e) => {
                let t = e.unescape().unwrap_or_default();
                let t = t.trim().to_string();
                if !t.is_empty() {
                    if let Some((_, obj)) = stack.last_mut() {
                        obj.insert("#text".into(), serde_json::Value::String(t));
                    }
                }
            }
            Event::End(_) => {
                if let Some((tag, obj)) = stack.pop() {
                    let val = serde_json::Value::Object(obj);
                    if let Some((_, parent)) = stack.last_mut() {
                        match parent.get_mut(&tag) {
                            Some(serde_json::Value::Array(a)) => a.push(val),
                            Some(old) => {
                                let o = old.clone();
                                *old = serde_json::json!([o, val]);
                            }
                            None => {
                                parent.insert(tag, val);
                            }
                        }
                    } else {
                        root = val;
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(root)
}

/// Pre-process yaml/xml/toml (or json/jsonl with a jaq filter) to a temporary
/// NDJSON file.  Returns (path, RAII guard).
fn preprocess_to_ndjson(entry: &FileEntry) -> Result<(PathBuf, TempFile), String> {
    let raw = entry.jaq_filter.as_deref().unwrap_or(".");
    let filter = if raw.trim().is_empty() { "." } else { raw };
    let ident = filter == ".";

    let mut rows: Vec<serde_json::Value> = Vec::new();
    for filename in &entry.filenames {
        let val = file_to_json_value(Path::new(filename), &entry.filetype)?;
        let items = if ident {
            match val {
                serde_json::Value::Array(a) => a,
                other => vec![other],
            }
        } else {
            apply_jaq(filter, &val)?
        };
        rows.extend(items);
    }

    let tmp = next_temp_path();
    let mut ndjson = String::with_capacity(rows.len() * 64);
    for row in &rows {
        ndjson.push_str(&serde_json::to_string(row).map_err(|e| e.to_string())?);
        ndjson.push('\n');
    }
    std::fs::write(&tmp, &ndjson).map_err(|e| e.to_string())?;
    let guard = TempFile(tmp.clone());
    Ok((tmp, guard))
}

// ─── View DDL builder ─────────────────────────────────────────────────────────

fn sql_path_list(filenames: &[String]) -> String {
    filenames
        .iter()
        .map(|f| format!("'{}'", f.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(", ")
}

fn entry_to_view_ddl(entry: &FileEntry, temp_guards: &mut Vec<TempFile>) -> Result<String, String> {
    let alias = &entry.alias;

    let has_filter = entry
        .jaq_filter
        .as_deref()
        .map(|f| !f.trim().is_empty() && f.trim() != ".")
        .unwrap_or(false);

    match entry.filetype.as_str() {
        "parquet" => {
            let list = sql_path_list(&entry.filenames);
            Ok(format!(
                "CREATE VIEW \"{alias}\" AS SELECT * FROM read_parquet([{list}]);"
            ))
        }
        "arrow" | "ipc" | "feather" => {
            // DuckDB reads Feather V2 via read_parquet; true Arrow IPC Streaming
            // will produce a clear error message if unsupported.
            let list = sql_path_list(&entry.filenames);
            Ok(format!(
                "CREATE VIEW \"{alias}\" AS SELECT * FROM read_parquet([{list}]);"
            ))
        }
        "csv" => {
            let list = sql_path_list(&entry.filenames);
            let sep = entry.sep.unwrap_or(b',') as char;
            Ok(format!(
                "CREATE VIEW \"{alias}\" AS SELECT * FROM read_csv([{list}], \
                 delim='{sep}', header=true, auto_detect=true);"
            ))
        }
        "json" if !has_filter => {
            let list = sql_path_list(&entry.filenames);
            Ok(format!(
                "CREATE VIEW \"{alias}\" AS SELECT * FROM read_json([{list}], auto_detect=true);"
            ))
        }
        "jsonl" | "ndjson" if !has_filter => {
            let list = sql_path_list(&entry.filenames);
            Ok(format!(
                "CREATE VIEW \"{alias}\" AS SELECT * FROM read_ndjson([{list}], auto_detect=true);"
            ))
        }
        // jaq-preprocessed paths (yaml/xml/toml always; json/jsonl when filtered)
        "json" | "jsonl" | "ndjson" | "yaml" | "yml" | "xml" | "toml" => {
            let (path, guard) = preprocess_to_ndjson(entry)?;
            let escaped = path.to_string_lossy().replace('\'', "''");
            temp_guards.push(guard);
            Ok(format!(
                "CREATE VIEW \"{alias}\" AS SELECT * FROM read_ndjson('{escaped}', auto_detect=true);"
            ))
        }
        // Attached databases
        "duckdb" => {
            let path = entry.filenames[0].replace('\'', "''");
            Ok(format!("ATTACH '{path}' AS \"{alias}\" (READ_ONLY);"))
        }
        "sqlite" => {
            let path = entry.filenames[0].replace('\'', "''");
            Ok(format!(
                "INSTALL sqlite; LOAD sqlite; \
                 ATTACH '{path}' AS \"{alias}\" (TYPE SQLITE, READ_ONLY);"
            ))
        }
        other => Err(format!("unknown filetype: {other}")),
    }
}

// ─── Connection builder ───────────────────────────────────────────────────────

fn build_connection(entries: &[FileEntry]) -> Result<(duckdb::Connection, Vec<TempFile>), String> {
    let conn = duckdb::Connection::open_in_memory().map_err(|e| e.to_string())?;
    let mut guards = Vec::new();
    for entry in entries {
        let ddl = entry_to_view_ddl(entry, &mut guards)?;
        conn.execute_batch(&ddl)
            .map_err(|e| format!("setup '{}': {}", entry.alias, e))?;
    }
    Ok((conn, guards))
}

fn parse_entries(entries_json: &str) -> Result<Vec<FileEntry>, String> {
    serde_json::from_str(entries_json)
        .map_err(|e| serde_json::json!({"err_msg": e.to_string()}).to_string())
}

// ─── Result serialisation ─────────────────────────────────────────────────────

fn generate_table(col_names: &[String], body: Vec<HashMap<String, String>>) -> String {
    let headers = col_names
        .iter()
        .map(|k| serde_json::json!({ "title": k, "key": k, "resizable": true }))
        .collect::<Vec<_>>();
    serde_json::to_string(&serde_json::json!({
        "col_count": col_names.len(),
        "row_count": body.len(),
        "headers":   headers,
        "body":      body,
    }))
    .unwrap()
}

// ─── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
fn execute_query(entries_json: &str, sql: &str) -> String {
    let entries = match parse_entries(entries_json) {
        Ok(e) => e,
        Err(s) => return s,
    };
    let (conn, _guards) = match build_connection(&entries) {
        Ok(r) => r,
        Err(e) => return err_json(&e),
    };

    let wrapped = format!("SELECT ROW_NUMBER() OVER () AS idx, * FROM ({sql}) __q");
    let mut stmt = match conn.prepare(&wrapped) {
        Ok(s) => s,
        Err(e) => return err_json(&e.to_string()),
    };

    // Use query() so we can retrieve column names from Rows::as_ref()
    let mut rows = match stmt.query([]) {
        Ok(r) => r,
        Err(e) => return err_json(&e.to_string()),
    };

    let col_names: Vec<String> = rows.as_ref().map(|s| s.column_names()).unwrap_or_default();

    let mut body: Vec<HashMap<String, String>> = Vec::new();
    loop {
        let row = match rows.next() {
            Ok(Some(r)) => r,
            Ok(None) => break,
            Err(e) => return err_json(&e.to_string()),
        };
        let mut map = HashMap::new();
        for (i, name) in col_names.iter().enumerate() {
            let val: Value = row.get(i).unwrap_or(Value::Null);
            map.insert(name.clone(), value_to_display(&val));
        }
        body.push(map);
    }

    generate_table(&col_names, body)
}

#[tauri::command]
fn get_schema(entries_json: &str) -> String {
    let entries = match parse_entries(entries_json) {
        Ok(e) => e,
        Err(s) => return s,
    };
    let (conn, _guards) = match build_connection(&entries) {
        Ok(r) => r,
        Err(e) => return err_json(&e),
    };

    let mut tables = Vec::new();
    for entry in &entries {
        let alias = &entry.alias;

        // Attached databases: list tables instead of column schema
        let query = if entry.filetype == "duckdb" || entry.filetype == "sqlite" {
            format!(
                "SELECT table_name, table_type FROM information_schema.tables \
                 WHERE table_catalog = '{alias}' AND table_schema = 'main' \
                 ORDER BY table_name"
            )
        } else {
            format!("DESCRIBE \"{alias}\"")
        };

        let cols: Vec<serde_json::Value> = conn
            .prepare(&query)
            .and_then(|mut s| {
                s.query_map([], |row| {
                    Ok(serde_json::json!({
                        "name":  row.get::<_, String>(0)?,
                        "dtype": row.get::<_, String>(1)?,
                    }))
                })
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
            })
            .unwrap_or_default();

        tables.push(serde_json::json!({ "alias": alias, "columns": cols }));
    }
    serde_json::json!({ "ok": true, "tables": tables }).to_string()
}

#[tauri::command]
fn export_parquet(entries_json: &str, sql: &str, dest: &str) -> String {
    let entries = match parse_entries(entries_json) {
        Ok(e) => e,
        Err(s) => return s,
    };
    let (conn, _guards) = match build_connection(&entries) {
        Ok(r) => r,
        Err(e) => return err_json(&e),
    };
    let dest_esc = dest.replace('\'', "''");
    match conn.execute_batch(&format!("COPY ({sql}) TO '{dest_esc}' (FORMAT PARQUET)")) {
        Ok(_) => serde_json::json!({ "ok": true }).to_string(),
        Err(e) => err_json(&e.to_string()),
    }
}

#[tauri::command]
fn export_csv_file(entries_json: &str, sql: &str, dest: &str, sep: u8) -> String {
    let entries = match parse_entries(entries_json) {
        Ok(e) => e,
        Err(s) => return s,
    };
    let (conn, _guards) = match build_connection(&entries) {
        Ok(r) => r,
        Err(e) => return err_json(&e),
    };
    let dest_esc = dest.replace('\'', "''");
    let delim = sep as char;
    match conn.execute_batch(&format!(
        "COPY ({sql}) TO '{dest_esc}' (FORMAT CSV, DELIMITER '{delim}', HEADER true)"
    )) {
        Ok(_) => serde_json::json!({ "ok": true }).to_string(),
        Err(e) => err_json(&e.to_string()),
    }
}

#[tauri::command]
fn write_text_file(content: &str, dest: &str) -> String {
    match std::fs::write(dest, content) {
        Ok(_) => serde_json::json!({ "ok": true }).to_string(),
        Err(e) => err_json(&e.to_string()),
    }
}

/// Preview jaq filter output for the JaqModal (first 8 rows + total count).
#[tauri::command]
fn preview_jaq(paths: Vec<String>, filetype: String, filter: String) -> String {
    let mut parts: Vec<serde_json::Value> = Vec::new();
    for p in &paths {
        match file_to_json_value(Path::new(p), &filetype) {
            Ok(v) => parts.push(v),
            Err(e) => return serde_json::json!({"err_msg": e}).to_string(),
        }
    }
    let root = if parts.len() == 1 {
        parts.remove(0)
    } else {
        serde_json::Value::Array(parts)
    };

    let filter_str = if filter.trim().is_empty() {
        "."
    } else {
        &filter
    };
    match apply_jaq(filter_str, &root) {
        Ok(rows) => {
            let total = rows.len();
            let preview = rows.into_iter().take(8).collect::<Vec<_>>();
            serde_json::json!({ "ok": true, "total": total, "preview": preview }).to_string()
        }
        Err(e) => serde_json::json!({"err_msg": e}).to_string(),
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
            preview_jaq,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
