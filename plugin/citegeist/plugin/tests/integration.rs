use std::collections::HashMap;
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
struct MyEntry {
    entry_type: String,
    entry_key: String,
    fields: HashMap<String, String>,
    parsed_names: HashMap<String, Vec<HashMap<String, String>>>,
}

/// Helper: call get_bib_map with defaults (keep_raw_names=true, sentence_case_titles=true).
fn parse_bib(bib: &str) -> HashMap<String, MyEntry> {
    let result_bytes = citegeist::get_bib_map(bib.as_bytes(), &[1], &[1]).unwrap();
    serde_cbor::from_slice(&result_bytes).unwrap()
}

/// Helper: call with keep_raw_names=false, sentence_case_titles=true.
fn parse_bib_no_raw_names(bib: &str) -> HashMap<String, MyEntry> {
    let result_bytes = citegeist::get_bib_map(bib.as_bytes(), &[0], &[1]).unwrap();
    serde_cbor::from_slice(&result_bytes).unwrap()
}

/// Helper: call with keep_raw_names=true, sentence_case_titles=false.
fn parse_bib_verbatim_titles(bib: &str) -> HashMap<String, MyEntry> {
    let result_bytes = citegeist::get_bib_map(bib.as_bytes(), &[1], &[0]).unwrap();
    serde_cbor::from_slice(&result_bytes).unwrap()
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

    assert!(entry.fields.get("url").unwrap().contains("aclanthology.org"));
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
    let result_bytes = citegeist::get_bib_map(bib.as_bytes(), &[], &[]).unwrap();
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
    assert_eq!(entry.fields.get("title").unwrap(), "Test title with Proper nouns");
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
    assert_eq!(entry.fields.get("title").unwrap(), "Test Title With Proper Nouns");
}
