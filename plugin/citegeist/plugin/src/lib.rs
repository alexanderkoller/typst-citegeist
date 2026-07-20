use biblatex::*;
use core::str;
use indexmap::IndexMap;
#[cfg(target_arch = "wasm32")]
use wasm_minimal_protocol::*;

#[cfg(target_arch = "wasm32")]
initiate_protocol!();

use serde_cbor::to_vec;
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
    parsed_names: IndexMap<String, Vec<MyPerson>>,
    parsed_dates: IndexMap<String, MyDate>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MyPerson {
    family: String,
    given: String,
    prefix: String,
    suffix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(rename = "prefix-initials", skip_serializing_if = "Option::is_none")]
    prefix_initials: Option<String>,
    #[serde(rename = "given-initials", skip_serializing_if = "Option::is_none")]
    given_initials: Option<String>,
    #[serde(rename = "use-prefix", skip_serializing_if = "Option::is_none")]
    use_prefix: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MyDate {
    kind: String,
    uncertain: bool,
    approximate: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<MyDatetime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<MyDatetime>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MyDatetime {
    year: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    month: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    day: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    time: Option<MyTime>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MyTime {
    hour: u8,
    minute: u8,
    second: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<MyTimeOffset>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MyTimeOffset {
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    positive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hours: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minutes: Option<u8>,
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
            convert_entry(entry, position, keep_raw_names, sentence_case_titles)?,
        );
    }

    to_vec(&ret).map_err(|e| format!("failed to serialize result: {e}"))
}

fn convert_entry(
    entry: &Entry,
    position: usize,
    keep_raw_names: bool,
    sentence_case_titles: bool,
) -> Result<MyEntry, String> {
    let mut ret = MyEntry {
        entry_type: entry.entry_type.to_string(),
        entry_key: entry.key.clone(),
        position,
        fields: IndexMap::with_capacity(entry.fields.len()),
        parsed_names: IndexMap::new(),
        parsed_dates: IndexMap::new(),
    };

    for (key, chunks) in &entry.fields {
        if NAME_FIELDS.contains(&key.as_str()) {
            if let Ok(names) = <Vec<Person> as Type>::from_chunks(chunks) {
                let parsed: Vec<MyPerson> = names.into_iter().map(person_to_map).collect();
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

    insert_date_fields(&mut ret.parsed_dates, entry)
        .map_err(|e| format!("invalid date in entry `{}`: {e}", entry.key))?;

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

    Ok(ret)
}

fn insert_date_fields(
    parsed_dates: &mut IndexMap<String, MyDate>,
    entry: &Entry,
) -> Result<(), String> {
    maybe_insert_date(parsed_dates, "date", entry.date())?;
    maybe_insert_date(parsed_dates, "eventdate", entry.event_date())?;
    maybe_insert_date(parsed_dates, "urldate", entry.url_date())?;
    maybe_insert_date(parsed_dates, "origdate", entry.orig_date())?;
    Ok(())
}

fn maybe_insert_date(
    parsed_dates: &mut IndexMap<String, MyDate>,
    key: &str,
    date: Result<PermissiveType<Date>, RetrievalError>,
) -> Result<(), String> {
    match date {
        Ok(PermissiveType::Typed(date)) => {
            parsed_dates.insert(key.into(), date_to_map(date));
            Ok(())
        }
        Ok(PermissiveType::Chunks(_)) => Err(format!("field `{key}` is not a valid date")),
        Err(RetrievalError::Missing(_)) => Ok(()),
        Err(err) => Err(format!("field `{key}` is not a valid date: {err}")),
    }
}

fn date_to_map(date: Date) -> MyDate {
    let (kind, start, end) = match date.value {
        DateValue::At(start) => ("at", Some(datetime_to_map(start)), None),
        DateValue::After(start) => ("after", Some(datetime_to_map(start)), None),
        DateValue::Before(end) => ("before", None, Some(datetime_to_map(end))),
        DateValue::Between(start, end) => (
            "between",
            Some(datetime_to_map(start)),
            Some(datetime_to_map(end)),
        ),
    };

    MyDate {
        kind: kind.into(),
        uncertain: date.uncertain,
        approximate: date.approximate,
        start,
        end,
    }
}

fn datetime_to_map(datetime: Datetime) -> MyDatetime {
    MyDatetime {
        year: datetime.year,
        month: datetime.month.map(|month| month + 1),
        day: datetime.day.map(|day| day + 1),
        time: datetime.time.map(time_to_map),
    }
}

fn time_to_map(time: Time) -> MyTime {
    MyTime {
        hour: time.hour,
        minute: time.minute,
        second: time.second,
        offset: time.offset.map(time_offset_to_map),
    }
}

fn time_offset_to_map(offset: TimeOffset) -> MyTimeOffset {
    match offset {
        TimeOffset::Utc => MyTimeOffset {
            kind: "utc".into(),
            positive: None,
            hours: None,
            minutes: None,
        },
        TimeOffset::Offset {
            positive,
            hours,
            minutes,
        } => MyTimeOffset {
            kind: "offset".into(),
            positive: Some(positive),
            hours: Some(hours),
            minutes: Some(minutes),
        },
    }
}

fn person_to_map(p: Person) -> MyPerson {
    MyPerson {
        family: p.name,
        given: p.given_name,
        prefix: p.prefix,
        suffix: p.suffix,
        id: p.id,
        prefix_initials: p.prefix_initials,
        given_initials: p.given_initials,
        use_prefix: p.use_prefix,
    }
}
