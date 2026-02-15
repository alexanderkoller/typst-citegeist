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
/// `options_u8` controls behaviour. Currently only the first byte is used:
///   - byte 0: `keep_raw_names` (1 = true, 0 = false; default = 1 if empty)
///
/// When `keep_raw_names` is false, name fields (author, editor, …) are only
/// returned as structured data in `parsed_names` and omitted from `fields`.
#[cfg_attr(target_arch = "wasm32", wasm_func)]
pub fn get_bib_map(bib_contents_u8: &[u8], options_u8: &[u8]) -> Result<Vec<u8>, String> {
    let keep_raw_names = match options_u8.first() {
        Some(&b) => b != 0,
        None => true,
    };

    let bib_contents = str::from_utf8(bib_contents_u8)
        .map_err(|e| format!("invalid UTF-8 in bibliography: {e}"))?;

    let bibliography = Bibliography::parse(bib_contents)
        .map_err(|e| format!("failed to parse bibliography: {e}"))?;

    let mut ret: HashMap<String, MyEntry> = HashMap::with_capacity(bibliography.len());

    for entry in bibliography.iter() {
        ret.insert(entry.key.clone(), convert_entry(entry, keep_raw_names));
    }

    to_vec(&ret).map_err(|e| format!("failed to serialize result: {e}"))
}

fn convert_entry(entry: &Entry, keep_raw_names: bool) -> MyEntry {
    let mut ret = MyEntry {
        entry_type: entry.entry_type.to_string(),
        entry_key: entry.key.clone(),
        fields: HashMap::with_capacity(entry.fields.len()),
        parsed_names: HashMap::new(),
    };

    for (key, chunks) in &entry.fields {
        if NAME_FIELDS.contains(&key.as_str()) {
            // Parse names into structured data.
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
            // Only store the verbatim string if requested.
            if keep_raw_names {
                ret.fields.insert(key.clone(), chunks.format_verbatim());
            }
        } else if key == "title" {
            ret.fields.insert(key.clone(), chunks.format_sentence());
        } else {
            ret.fields.insert(key.clone(), chunks.format_verbatim());
        }
    }

    // Fall back to the accessor if title wasn't a direct field
    // (entry.title() resolves aliases like maintitle).
    if !ret.fields.contains_key("title") {
        if let Ok(title) = entry.title() {
            ret.fields.insert("title".into(), title.format_sentence());
        }
    }

    ret
}
