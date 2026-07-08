use biblatex::*;
use core::str;
use indexmap::IndexMap;
use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use wasm_minimal_protocol::*;

#[cfg(target_arch = "wasm32")]
initiate_protocol!();

use serde_cbor::to_vec;
use serde_cbor::value::Value;
use serde_derive::{Deserialize, Serialize};

mod diagnostics;
use diagnostics::format_parse_error;

const NAME_FIELDS: &[&str] = &[
    "afterword",
    "annotator",
    "author",
    "bookauthor",
    "commentator",
    "editor",
    "editora",
    "editorb",
    "editorc",
    "foreword",
    "holder",
    "introduction",
    "shortauthor",
    "shorteditor",
    "translator",
];

#[derive(Debug, Serialize, Deserialize)]
struct MyEntry {
    entry_type: String,
    entry_key: String,
    position: usize,
    fields: IndexMap<String, String>,
    parsed_names: IndexMap<String, Vec<HashMap<String, Value>>>,
}

/// Main entry point for the plugin.
///
/// Parameters:
///   - `bib_contents_u8`: UTF-8 encoded bibliography content
///   - `keep_raw_names_u8`: single byte; 1 = keep raw name strings in `fields`,
///      0 = omit them (default: 1 if empty)
///   - `sentence_case_titles_u8`: single byte; 1 = format titles in sentence case,
///      0 = keep titles verbatim (default: 1 if empty)
///   - `on_duplicate_u8`: single byte controlling duplicate-key handling
///      (default: 0 if empty):
///        * 0 = error (the whole parse fails, as before);
///        * 1 = keep the first entry with a given key, drop later duplicates;
///        * 2 = keep the last entry with a given key, drop earlier duplicates.
///   - `source_name_u8`: optional UTF-8 encoded source label for diagnostics.
#[cfg_attr(target_arch = "wasm32", wasm_func)]
pub fn get_bib_map(
    bib_contents_u8: &[u8],
    keep_raw_names_u8: &[u8],
    sentence_case_titles_u8: &[u8],
    on_duplicate_u8: &[u8],
    source_name_u8: &[u8],
) -> Result<Vec<u8>, String> {
    let keep_raw_names = match keep_raw_names_u8.first() {
        Some(&b) => b != 0,
        None => true,
    };
    let sentence_case_titles = match sentence_case_titles_u8.first() {
        Some(&b) => b != 0,
        None => true,
    };
    let on_duplicate = on_duplicate_u8.first().copied().unwrap_or(0);

    let bib_contents = str::from_utf8(bib_contents_u8)
        .map_err(|e| format!("invalid UTF-8 in bibliography: {e}"))?;
    let source_name = if source_name_u8.is_empty() {
        None
    } else {
        Some(
            str::from_utf8(source_name_u8)
                .map_err(|e| format!("invalid UTF-8 in bibliography source name: {e}"))?,
        )
    };

    let bibliography = if on_duplicate == 0 {
        // Default: hard error on a duplicate key (pre-0.3.0 behaviour).
        Bibliography::parse(bib_contents)
            .map_err(|e| format_parse_error(bib_contents, source_name, &e))?
    } else {
        // Tolerant modes: dedup at the raw level (before `from_raw`, which is
        // where the duplicate-key check lives), then build normally so xdata /
        // crossref resolution still runs.
        let mut raw = RawBibliography::parse(bib_contents)
            .map_err(|e| format_parse_error(bib_contents, source_name, &e))?;
        let mut seen = std::collections::HashSet::new();

        if on_duplicate == 2 {
            // keep last: walk from the end, keep first-seen-from-the-back, restore order
            let mut kept: Vec<_> = raw
                .entries
                .into_iter()
                .rev()
                .filter(|e| seen.insert(e.v.key.v.to_string()))
                .collect();
            kept.reverse();
            raw.entries = kept;
        } else {
            // on_duplicate == 1
            // keep first (any non-zero value other than 2)
            raw.entries.retain(|e| seen.insert(e.v.key.v.to_string()));
        }

        Bibliography::from_raw(raw)
            .map_err(|e| format_parse_error(bib_contents, source_name, &e))?
    };

    // IndexMap preserves source order from `bibliography.iter()`
    // to match biblatex's internal Vec<Entry>.
    let mut ret: IndexMap<String, MyEntry> = IndexMap::with_capacity(bibliography.len());

    for (position, entry) in bibliography.iter().enumerate() {
        ret.insert(
            entry.key.clone(),
            convert_entry(entry, position, keep_raw_names, sentence_case_titles),
        );
    }

    to_vec(&ret).map_err(|e| format!("failed to serialize result: {e}"))
}

fn convert_entry(
    entry: &Entry,
    position: usize,
    keep_raw_names: bool,
    sentence_case_titles: bool,
) -> MyEntry {
    let mut ret = MyEntry {
        entry_type: entry.entry_type.to_string(),
        entry_key: entry.key.clone(),
        position,
        fields: IndexMap::with_capacity(entry.fields.len()),
        parsed_names: IndexMap::new(),
    };

    for (key, chunks) in &entry.fields {
        if NAME_FIELDS.contains(&key.as_str()) {
            if let Ok(names) = <Vec<Person> as Type>::from_chunks(chunks) {
                let parsed: Vec<HashMap<String, Value>> =
                    names.into_iter().map(person_to_map).collect();
                ret.parsed_names.insert(key.clone(), parsed);
            }
            if keep_raw_names {
                ret.fields.insert(key.clone(), chunks.format_verbatim());
            }
        } else if key == "title" {
            if sentence_case_titles {
                ret.fields.insert(key.clone(), chunks.format_sentence());
            } else {
                ret.fields.insert(key.clone(), chunks.format_verbatim());
            }
        } else {
            ret.fields.insert(key.clone(), chunks.format_verbatim());
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

fn text(value: String) -> Value {
    Value::Text(value)
}

fn person_to_map(p: Person) -> HashMap<String, Value> {
    let mut name = HashMap::from([
        ("family".into(), text(p.name)),
        ("given".into(), text(p.given_name)),
        ("prefix".into(), text(p.prefix)),
        ("suffix".into(), text(p.suffix)),
    ]);

    if let Some(id) = p.id {
        name.insert("id".into(), text(id));
    }
    if let Some(prefix_initials) = p.prefix_initials {
        name.insert("prefix-initials".into(), text(prefix_initials));
    }
    if let Some(given_initials) = p.given_initials {
        name.insert("given-initials".into(), text(given_initials));
    }
    if let Some(use_prefix) = p.use_prefix {
        name.insert("use-prefix".into(), Value::Bool(use_prefix));
    }

    name
}
