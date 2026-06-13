#[cfg(target_arch = "wasm32")]
use wasm_minimal_protocol::*;
use biblatex::*;
use core::str;
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
initiate_protocol!();

use serde_derive::{Deserialize, Serialize};
use serde_cbor::to_vec;

const NAME_FIELDS: &[&str] = &[
    "afterword", "annotator", "author", "bookauthor", "commentator",
    "editor", "editora", "editorb", "editorc", "foreword", "holder",
    "introduction", "shortauthor", "shorteditor", "translator",
];

#[derive(Debug, Serialize, Deserialize)]
struct MyEntry {
    entry_type: String,
    entry_key: String,
    fields: HashMap<String, String>,
    parsed_names: HashMap<String, Vec<HashMap<String, String>>>,
}

/// Main entry point for the plugin.
///
/// Parameters:
///   - `bib_contents_u8`: UTF-8 encoded bibliography content
///   - `keep_raw_names_u8`: single byte; 1 = keep raw name strings in `fields`,
///      0 = omit them (default: 1 if empty)
///   - `sentence_case_titles_u8`: single byte; 1 = format titles in sentence case,
///      0 = keep titles verbatim (default: 1 if empty)
///   - `verbatim_u8`: single byte; 1 = return every field value byte-for-byte
///      as written in the source (no LaTeX/escape interpretation, no brace
///      stripping, no sentence casing), 0 = interpret as before
///      (default: 0 if empty).
#[cfg_attr(target_arch = "wasm32", wasm_func)]
pub fn get_bib_map(
    bib_contents_u8: &[u8],
    keep_raw_names_u8: &[u8],
    sentence_case_titles_u8: &[u8],
    verbatim_u8: &[u8],
) -> Result<Vec<u8>, String> {
    let keep_raw_names = match keep_raw_names_u8.first() {
        Some(&b) => b != 0,
        None => true,
    };
    let sentence_case_titles = match sentence_case_titles_u8.first() {
        Some(&b) => b != 0,
        None => true,
    };
    let verbatim = matches!(verbatim_u8.first(), Some(&b) if b != 0);

    let bib_contents = str::from_utf8(bib_contents_u8)
        .map_err(|e| format!("invalid UTF-8 in bibliography: {e}"))?;

    let bibliography = Bibliography::parse(bib_contents)
        .map_err(|e| format!("failed to parse bibliography: {e}"))?;

    let mut ret: HashMap<String, MyEntry> = HashMap::with_capacity(bibliography.len());

    for entry in bibliography.iter() {
        ret.insert(
            entry.key.clone(),
            convert_entry(entry, keep_raw_names, sentence_case_titles, verbatim, bib_contents),
        );
    }

    to_vec(&ret).map_err(|e| format!("failed to serialize result: {e}"))
}

/// Byte-for-byte source slice covering a field's chunks (verbatim mode):
/// from the start of the first chunk's span to the end of the last chunk's span.
/// Returns the raw `.bib` text, with no escape interpretation or brace stripping.
fn raw_source(chunks: &Chunks, src: &str) -> String {
    match (chunks.first(), chunks.last()) {
        (Some(first), Some(last)) => {
            src.get(first.span.start..last.span.end).unwrap_or("").to_string()
        }
        _ => String::new(),
    }
}

fn convert_entry(
    entry: &Entry,
    keep_raw_names: bool,
    sentence_case_titles: bool,
    verbatim: bool,
    src: &str,
) -> MyEntry {
    let mut ret = MyEntry {
        entry_type: entry.entry_type.to_string(),
        entry_key: entry.key.clone(),
        fields: HashMap::with_capacity(entry.fields.len()),
        parsed_names: HashMap::new(),
    };

    for (key, chunks) in &entry.fields {
        if NAME_FIELDS.contains(&key.as_str()) {
            if let Ok(names) = <Vec<Person> as Type>::from_chunks(chunks) {
                let parsed: Vec<HashMap<String, String>> = names
                    .into_iter()
                    .map(|p| {
                        HashMap::from([
                            ("family".into(), p.name),
                            ("given".into(), p.given_name),
                            ("prefix".into(), p.prefix),
                            ("suffix".into(), p.suffix),
                        ])
                    })
                    .collect();
                ret.parsed_names.insert(key.clone(), parsed);
            }
            if keep_raw_names {
                let v = if verbatim { raw_source(chunks, src) } else { chunks.format_verbatim() };
                ret.fields.insert(key.clone(), v);
            }
        } else if key == "title" {
            let v = if verbatim {
                raw_source(chunks, src)
            } else if sentence_case_titles {
                chunks.format_sentence()
            } else {
                chunks.format_verbatim()
            };
            ret.fields.insert(key.clone(), v);
        } else {
            let v = if verbatim { raw_source(chunks, src) } else { chunks.format_verbatim() };
            ret.fields.insert(key.clone(), v);
        }
    }

    // Fall back to the accessor if title wasn't a direct field
    // (entry.title() resolves aliases like maintitle).
    if !ret.fields.contains_key("title") {
        if let Ok(title) = entry.title() {
            if sentence_case_titles {
                ret.fields.insert("title".into(), title.format_sentence());
            } else {
                ret.fields.insert("title".into(), title.format_verbatim());
            }
        }
    }

    ret
}
