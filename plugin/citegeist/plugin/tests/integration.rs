use serde_cbor::value::Value;
use serde_derive::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct MyEntry {
    entry_type: String,
    entry_key: String,
    position: usize,
    fields: HashMap<String, String>,
    parsed_names: HashMap<String, Vec<HashMap<String, String>>>,
}

#[derive(Debug, Deserialize)]
struct MyEntryWithValueNames {
    parsed_names: HashMap<String, Vec<HashMap<String, Value>>>,
}

/// Helper: call get_bib_map with defaults (keep_raw_names=true, sentence_case_titles=true).
fn parse_bib(bib: &str) -> HashMap<String, MyEntry> {
    let result_bytes = citegeist::get_bib_map(bib.as_bytes(), &[1], &[1], &[], &[]).unwrap();
    serde_cbor::from_slice(&result_bytes).unwrap()
}

/// Helper: call with keep_raw_names=false, sentence_case_titles=true.
fn parse_bib_no_raw_names(bib: &str) -> HashMap<String, MyEntry> {
    let result_bytes = citegeist::get_bib_map(bib.as_bytes(), &[0], &[1], &[], &[]).unwrap();
    serde_cbor::from_slice(&result_bytes).unwrap()
}

/// Helper: call with keep_raw_names=true, sentence_case_titles=false.
fn parse_bib_verbatim_titles(bib: &str) -> HashMap<String, MyEntry> {
    let result_bytes = citegeist::get_bib_map(bib.as_bytes(), &[1], &[0], &[], &[]).unwrap();
    serde_cbor::from_slice(&result_bytes).unwrap()
}

fn parse_bib_value_names(bib: &str) -> HashMap<String, MyEntryWithValueNames> {
    let result_bytes = citegeist::get_bib_map(bib.as_bytes(), &[1], &[1], &[], &[]).unwrap();
    serde_cbor::from_slice(&result_bytes).unwrap()
}

fn parse_bib_error(bib: &str, on_duplicate: &[u8], source: &[u8]) -> String {
    citegeist::get_bib_map(bib.as_bytes(), &[1], &[1], on_duplicate, source).unwrap_err()
}

fn assert_common_diagnostic(message: &str) {
    assert!(
        message.contains("failed to parse bibliography"),
        "missing parse prefix:\n{message}"
    );
    assert!(message.contains("line "), "missing line:\n{message}");
    assert!(message.contains("column "), "missing column:\n{message}");
    assert!(message.contains("byte "), "missing byte span:\n{message}");
    assert!(message.contains(" | "), "missing excerpt:\n{message}");
    assert!(message.contains("^"), "missing caret:\n{message}");
}

#[test]
fn test_parse_simple_bib() {
    let bib = r#"
@article{test-article,
    title = "Test Title",
    author = "Doe, John",
    year = "2024",
    journal = "Test Journal",
}
"#;
    let result = parse_bib(bib);

    assert_eq!(result.len(), 1);
    let entry = result.get("test-article").unwrap();
    assert_eq!(entry.entry_type, "article");
    assert_eq!(entry.entry_key, "test-article");
    assert_eq!(entry.position, 0);
    assert_eq!(entry.fields.get("title").unwrap(), "Test title");
    assert_eq!(entry.fields.get("year").unwrap(), "2024");

    // Check parsed author
    let authors = entry.parsed_names.get("author").unwrap();
    assert_eq!(authors.len(), 1);
    assert_eq!(authors[0].get("family").unwrap(), "Doe");
    assert_eq!(authors[0].get("given").unwrap(), "John");

    // With keep_raw_names, the raw author string is also in fields
    assert!(entry.fields.contains_key("author"));
}

#[test]
fn test_parse_multiple_authors() {
    let bib = r#"
@inproceedings{multi-author,
    title = "Collaborative Research",
    author = "Smith, Alice and Jones, Bob and Williams, Carol",
    year = "2023",
    booktitle = "Proceedings of Test Conference",
}
"#;
    let result = parse_bib(bib);

    let entry = result.get("multi-author").unwrap();
    let authors = entry.parsed_names.get("author").unwrap();

    assert_eq!(authors.len(), 3);
    assert_eq!(authors[0].get("family").unwrap(), "Smith");
    assert_eq!(authors[1].get("family").unwrap(), "Jones");
    assert_eq!(authors[2].get("family").unwrap(), "Williams");
}

#[test]
fn test_extended_name_options_are_exposed_when_present() {
    let bib = r#"
@book{extended-names,
    title = "Extended Names",
    author = "given=Simon, prefix=de, family=Beumont, useprefix=true and given=Jean Pierre Simon, given-i=JPS, prefix=de la, prefix-i=d, family=Rousse, id=rousse-jps",
    editor = "Doe, Jane",
    year = "2026",
}
"#;
    let result = parse_bib_value_names(bib);
    let entry = result.get("extended-names").unwrap();

    let authors = entry.parsed_names.get("author").unwrap();
    assert_eq!(authors.len(), 2);

    assert_eq!(
        authors[0].get("family"),
        Some(&Value::Text("Beumont".into()))
    );
    assert_eq!(authors[0].get("given"), Some(&Value::Text("Simon".into())));
    assert_eq!(authors[0].get("prefix"), Some(&Value::Text("de".into())));
    assert_eq!(authors[0].get("use-prefix"), Some(&Value::Bool(true)));
    assert!(!authors[0].contains_key("given-initials"));
    assert!(!authors[0].contains_key("prefix-initials"));
    assert!(!authors[0].contains_key("id"));

    assert_eq!(
        authors[1].get("family"),
        Some(&Value::Text("Rousse".into()))
    );
    assert_eq!(
        authors[1].get("given"),
        Some(&Value::Text("Jean Pierre Simon".into()))
    );
    assert_eq!(authors[1].get("prefix"), Some(&Value::Text("de la".into())));
    assert_eq!(
        authors[1].get("given-initials"),
        Some(&Value::Text("JPS".into()))
    );
    assert_eq!(
        authors[1].get("prefix-initials"),
        Some(&Value::Text("d".into()))
    );
    assert_eq!(
        authors[1].get("id"),
        Some(&Value::Text("rousse-jps".into()))
    );
    assert!(!authors[1].contains_key("use-prefix"));

    let editors = entry.parsed_names.get("editor").unwrap();
    assert_eq!(editors[0].get("family"), Some(&Value::Text("Doe".into())));
    assert!(!editors[0].contains_key("given-initials"));
    assert!(!editors[0].contains_key("prefix-initials"));
    assert!(!editors[0].contains_key("use-prefix"));
    assert!(!editors[0].contains_key("id"));
}

#[test]
fn test_parse_with_editor() {
    let bib = r#"
@inproceedings{with-editor,
    title = "Edited Work",
    author = "Writer, William",
    editor = "Editor, Edward",
    year = "2022",
    booktitle = "Edited Volume",
}
"#;
    let result = parse_bib(bib);

    let entry = result.get("with-editor").unwrap();

    let authors = entry.parsed_names.get("author").unwrap();
    assert_eq!(authors[0].get("family").unwrap(), "Writer");

    let editors = entry.parsed_names.get("editor").unwrap();
    assert_eq!(editors[0].get("family").unwrap(), "Editor");
}

#[test]
fn test_parse_multiple_entries() {
    let bib = r#"
@article{first,
    title = "First Article",
    author = "One, Author",
    year = "2020",
    journal = "Journal A",
}

@book{second,
    title = "Second Book",
    author = "Two, Author",
    year = "2021",
    publisher = "Publisher B",
}

@misc{third,
    title = "Third Misc",
    author = "Three, Author",
    year = "2022",
}
"#;
    let result = parse_bib(bib);

    assert_eq!(result.len(), 3);
    assert!(result.contains_key("first"));
    assert!(result.contains_key("second"));
    assert!(result.contains_key("third"));

    assert_eq!(result.get("first").unwrap().entry_type, "article");
    assert_eq!(result.get("second").unwrap().entry_type, "book");
    assert_eq!(result.get("third").unwrap().entry_type, "misc");
    assert_eq!(result.get("first").unwrap().position, 0);
    assert_eq!(result.get("second").unwrap().position, 1);
    assert_eq!(result.get("third").unwrap().position, 2);
}

#[test]
fn test_parse_with_special_characters() {
    let bib = r#"
@article{special-chars,
    title = "Testing {LaTeX} Braces and \"Quotes\"",
    author = "O'Brien, Patrick",
    year = "2024",
    journal = "Test Journal",
}
"#;
    let result = parse_bib(bib);

    let entry = result.get("special-chars").unwrap();
    assert!(entry.fields.contains_key("title"));
}

#[test]
fn test_parse_empty_bibliography() {
    let bib = "";
    let result = parse_bib(bib);
    assert!(result.is_empty());
}

#[test]
fn test_real_world_entry() {
    let bib = r#"
@inproceedings{bender-koller-2020-climbing,
    title = "Climbing towards {NLU}: {On} Meaning, Form, and Understanding in the Age of Data",
    author = "Bender, Emily M.  and Koller, Alexander",
    editor = "Jurafsky, Dan  and Chai, Joyce",
    booktitle = "Proceedings of the 58th Annual Meeting of the ACL",
    year = "2020",
    url = "https://aclanthology.org/2020.acl-main.463",
    doi = "10.18653/v1/2020.acl-main.463",
    pages = "5185--5198",
}
"#;
    let result = parse_bib(bib);

    let entry = result.get("bender-koller-2020-climbing").unwrap();
    assert_eq!(entry.entry_type, "inproceedings");

    let authors = entry.parsed_names.get("author").unwrap();
    assert_eq!(authors.len(), 2);
    assert_eq!(authors[0].get("family").unwrap(), "Bender");
    assert_eq!(authors[0].get("given").unwrap(), "Emily M.");
    assert_eq!(authors[1].get("family").unwrap(), "Koller");

    let editors = entry.parsed_names.get("editor").unwrap();
    assert_eq!(editors.len(), 2);

    assert!(entry
        .fields
        .get("url")
        .unwrap()
        .contains("aclanthology.org"));
}

#[test]
fn test_keep_raw_names_true() {
    let bib = r#"
@article{test,
    title = "Test",
    author = "Doe, John",
    editor = "Smith, Jane",
    year = "2024",
    journal = "Test Journal",
}
"#;
    let result = parse_bib(bib);
    let entry = result.get("test").unwrap();

    assert!(entry.fields.contains_key("author"));
    assert!(entry.fields.contains_key("editor"));
    assert!(entry.parsed_names.contains_key("author"));
    assert!(entry.parsed_names.contains_key("editor"));
}

#[test]
fn test_keep_raw_names_false() {
    let bib = r#"
@article{test,
    title = "Test",
    author = "Doe, John",
    editor = "Smith, Jane",
    year = "2024",
    journal = "Test Journal",
}
"#;
    let result = parse_bib_no_raw_names(bib);
    let entry = result.get("test").unwrap();

    assert!(!entry.fields.contains_key("author"));
    assert!(!entry.fields.contains_key("editor"));
    assert!(entry.parsed_names.contains_key("author"));
    assert!(entry.parsed_names.contains_key("editor"));
    assert!(entry.fields.contains_key("year"));
    assert!(entry.fields.contains_key("title"));
}

#[test]
fn test_default_options() {
    let bib = r#"
@article{test,
    title = "Test Title",
    author = "Doe, John",
    year = "2024",
    journal = "Test Journal",
}
"#;
    // Empty options = defaults (keep_raw_names=true, sentence_case_titles=true)
    let result_bytes = citegeist::get_bib_map(bib.as_bytes(), &[], &[], &[], &[]).unwrap();
    let result: HashMap<String, MyEntry> = serde_cbor::from_slice(&result_bytes).unwrap();
    let entry = result.get("test").unwrap();

    assert!(entry.fields.contains_key("author"));
    assert!(entry.parsed_names.contains_key("author"));
    // Default is sentence case
    assert_eq!(entry.fields.get("title").unwrap(), "Test title");
}

#[test]
fn test_sentence_case_titles_true() {
    let bib = r#"
@article{test,
    title = "Test Title With {Proper} Nouns",
    year = "2024",
    journal = "Test Journal",
}
"#;
    let result = parse_bib(bib);
    let entry = result.get("test").unwrap();

    // sentence case: first char uppercase, rest lowercase, braced text preserved
    assert_eq!(
        entry.fields.get("title").unwrap(),
        "Test title with Proper nouns"
    );
}

#[test]
fn test_sentence_case_titles_false() {
    let bib = r#"
@article{test,
    title = "Test Title With {Proper} Nouns",
    year = "2024",
    journal = "Test Journal",
}
"#;
    let result = parse_bib_verbatim_titles(bib);
    let entry = result.get("test").unwrap();

    // verbatim: preserved as-is (braces stripped but case unchanged)
    assert_eq!(
        entry.fields.get("title").unwrap(),
        "Test Title With Proper Nouns"
    );
}

// ---- order preservation (entries returned in source order) ----

/// Helper: deserialize into an order-preserving map to assert entry order.
fn parse_bib_ordered(bib: &str) -> indexmap::IndexMap<String, MyEntry> {
    let result_bytes = citegeist::get_bib_map(bib.as_bytes(), &[1], &[1], &[], &[]).unwrap();
    serde_cbor::from_slice(&result_bytes).unwrap()
}

#[test]
fn test_entry_order_preserved() {
    // Mixed entry types and non-alphabetical keys: a HashMap would reorder these.
    let bib = r#"
@article{zebra, title = {Z}, author = {A}, year = {2020}}
@book{alpha,    title = {A}, author = {B}, year = {2021}}
@inproceedings{mango, title = {M}, author = {C}, year = {2022}}
@book{beta,     title = {Be}, author = {D}, year = {2023}}
@misc{xray,     title = {X}, author = {E}, year = {2024}}
"#;
    let result = parse_bib_ordered(bib);
    let order: Vec<&str> = result.keys().map(|s| s.as_str()).collect();
    assert_eq!(order, vec!["zebra", "alpha", "mango", "beta", "xray"]);

    let positions: Vec<usize> = result.values().map(|entry| entry.position).collect();
    assert_eq!(positions, vec![0, 1, 2, 3, 4]);
}

// ---- duplicate-key handling ----

#[test]
fn test_duplicate_key_errors_by_default() {
    let bib = "@book{k, title={First}, author={A}, year={2020}}\n\
               @book{k, title={Second}, author={B}, year={2021}}";
    // on_duplicate = 0 (empty) -> hard error, unchanged behaviour.
    assert!(citegeist::get_bib_map(bib.as_bytes(), &[1], &[1], &[], &[]).is_err());
    assert!(citegeist::get_bib_map(bib.as_bytes(), &[1], &[1], &[0], &[]).is_err());
}

#[test]
fn test_missing_comma_diagnostic() {
    let bib = r#"@inproceedings{smith2024demo,
  title = {A Demo}
  pages = {7118-7118}
}"#;
    let message = parse_bib_error(bib, &[], &[]);

    assert_common_diagnostic(&message);
    assert!(message.contains("expected comma"), "{message}");
    assert!(
        message.contains("while parsing @inproceedings{smith2024demo}"),
        "{message}"
    );
    assert!(message.contains("pages = {7118-7118}"), "{message}");
    assert!(
        message.contains("hint: BibTeX fields must be separated by commas"),
        "{message}"
    );
}

#[test]
fn test_entry_context_ignores_at_sign_in_field_value() {
    let bib = r#"@article{real,
  title = {Email @book{fake}},
  year = {2024}
  journal = {Journal}
}"#;
    let message = parse_bib_error(bib, &[], &[]);

    assert_common_diagnostic(&message);
    assert!(
        message.contains("while parsing @article{real}"),
        "{message}"
    );
    assert!(!message.contains("while parsing @book{fake}"), "{message}");
}

#[test]
fn test_entry_context_ignores_at_sign_in_comment() {
    let bib = r#"@article{real,
  title = {A Demo}, % @book{fake}
  year = {2024}
  journal = {Journal}
}"#;
    let message = parse_bib_error(bib, &[], &[]);

    assert_common_diagnostic(&message);
    assert!(
        message.contains("while parsing @article{real}"),
        "{message}"
    );
    assert!(!message.contains("while parsing @book{fake}"), "{message}");
}

#[test]
fn test_unexpected_eof_diagnostic() {
    let bib = r#"@article{k,
  title = {A Demo"#;
    let message = parse_bib_error(bib, &[], &[]);

    assert_common_diagnostic(&message);
    assert!(message.contains("unexpected end of file"), "{message}");
    assert!(message.contains("while parsing @article{k}"), "{message}");
    assert!(
        message.contains("hint: The bibliography ended"),
        "{message}"
    );
}

#[test]
fn test_duplicate_key_diagnostic() {
    let bib = "@book{k, title={First}, author={A}, year={2020}}\n\
               @book{k, title={Second}, author={B}, year={2021}}";
    let message = parse_bib_error(bib, &[], &[]);

    assert_common_diagnostic(&message);
    assert!(message.contains("duplicate key \"k\""), "{message}");
    assert!(message.contains("while parsing @book{k}"), "{message}");
    assert!(
        message.contains("hint: Citation keys must be unique"),
        "{message}"
    );
}

#[test]
fn test_unknown_abbreviation_diagnostic() {
    let bib = r#"@article{k,
  title = not_defined_abbrev,
  year = {2024}
}"#;
    let message = parse_bib_error(bib, &[], &[]);

    assert_common_diagnostic(&message);
    assert!(
        message.contains("unknown abbreviation \"not_defined_abbrev\""),
        "{message}"
    );
    assert!(message.contains("while parsing @article{k}"), "{message}");
    assert!(
        message.contains("hint: Define the abbreviation"),
        "{message}"
    );
}

#[test]
fn test_source_name_included_in_diagnostic() {
    let bib = r#"@article{k,
  title = {A Demo}
  year = {2024}
}"#;
    let message = parse_bib_error(bib, &[], b"bibs/paper.bib");

    assert_common_diagnostic(&message);
    assert!(
        message.starts_with("failed to parse bibliography in bibs/paper.bib\n"),
        "{message}"
    );
}

#[test]
fn test_duplicate_key_keep_first() {
    let bib = "@book{k, title={First}, author={A}, year={2020}}\n\
               @book{k, title={Second}, author={B}, year={2021}}";
    let bytes = citegeist::get_bib_map(bib.as_bytes(), &[1], &[1], &[1], &[]).unwrap();
    let result: HashMap<String, MyEntry> = serde_cbor::from_slice(&bytes).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result["k"].fields["title"], "First");
    assert_eq!(result["k"].position, 0);
}

#[test]
fn test_duplicate_key_keep_last() {
    let bib = "@book{k, title={First}, author={A}, year={2020}}\n\
               @book{k, title={Second}, author={B}, year={2021}}";
    let bytes = citegeist::get_bib_map(bib.as_bytes(), &[1], &[1], &[2], &[]).unwrap();
    let result: HashMap<String, MyEntry> = serde_cbor::from_slice(&bytes).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result["k"].fields["title"], "Second");
    assert_eq!(result["k"].position, 0);
}

#[test]
fn test_positions_are_renumbered_after_deduplication() {
    let bib = "@book{a, title={A0}, author={A}, year={2020}}\n\
               @book{k, title={K0}, author={K}, year={2020}}\n\
               @book{b, title={B0}, author={B}, year={2020}}\n\
               @book{k, title={K1}, author={K}, year={2021}}\n\
               @book{c, title={C0}, author={C}, year={2020}}";
    let bytes = citegeist::get_bib_map(bib.as_bytes(), &[1], &[1], &[2], &[]).unwrap();
    let result: indexmap::IndexMap<String, MyEntry> = serde_cbor::from_slice(&bytes).unwrap();

    let keys: Vec<&str> = result.keys().map(|key| key.as_str()).collect();
    let positions: Vec<usize> = result.values().map(|entry| entry.position).collect();

    assert_eq!(keys, vec!["a", "b", "k", "c"]);
    assert_eq!(positions, vec![0, 1, 2, 3]);
    assert_eq!(result["k"].fields["title"], "K1");
}
