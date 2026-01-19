use wasm_minimal_protocol::*;
use biblatex::*;
use core::str;
use std::collections::BTreeMap;

// Importing this (or something else that accesses the console) will cause weird errors of the form
// error: cannot find definition for import __wbindgen_placeholder__::__wbindgen_describe with type Func(FuncType { params: [I32], results: [] })
// use wasm_bindgen::prelude::*;
// use web_sys::console;

initiate_protocol!();

use serde_derive::{Deserialize, Serialize};
use serde_cbor::to_vec;



// must pass bib contents as parameter here because the plugin
// runs in a sandbox that does not allow file access

#[derive(Debug, Serialize, Deserialize)]
struct MyEntry {
    entry_type: String,
    entry_key: String,
    fields: BTreeMap<String, String>,
    parsed_names: BTreeMap<String, Vec<BTreeMap<String,String>>>
}


#[wasm_func]
pub fn get_bib_map(bib_contents_u8: &[u8]) -> Result<Vec<u8>, String> {
    let bib_contents = str::from_utf8(bib_contents_u8)
        .map_err(|e| format!("invalid UTF-8 in bibliography: {e}"))?;
    
    let bibliography = Bibliography::parse(bib_contents)
        .map_err(|e| format!("failed to parse bibliography: {e}"))?;

    let mut ret: BTreeMap<String, MyEntry> = BTreeMap::new();
    
    for entry in bibliography.iter() {
        ret.insert(entry.key.clone(), convert_entry(entry));
    }

    to_vec(&ret).map_err(|e| format!("failed to serialize result: {e}"))
}


fn convert_chunks_to_string(chunks: &Chunks) -> String {
    // You can choose the formatting method here; for example, to use `format_verbatim`
    chunks.format_verbatim()
}

fn insert_converted_fields(
    original_map: BTreeMap<String, Chunks>, 
    target_map: &mut BTreeMap<String, String>
) {
    for (key, value) in original_map {
        // Convert the `Chunks` into a `String`
        let converted_value = convert_chunks_to_string(&value);
        
        // Insert into the target map
        target_map.insert(key, converted_value);
    }
}

fn create_entry(entry_type: String, entry_key: String) -> MyEntry {
    return MyEntry { 
        entry_key: entry_key, 
        entry_type: entry_type, 
        fields: BTreeMap::new(),
        parsed_names: BTreeMap::new()
    };
}

// fn first_last(person: &Person) -> String {
//     return format!("{} {}", person.given_name, person.name);
// }

fn convert_person(person:&Person) -> BTreeMap<String, String> {
    let mut ret = BTreeMap::<String, String>::new();
    ret.insert("family".to_string(), String::from(person.name.clone()));
    ret.insert("given".to_string(), String::from(person.given_name.clone()));
    ret.insert("prefix".to_string(), String::from(person.prefix.clone()));
    ret.insert("suffix".to_string(), String::from(person.suffix.clone()));

    return ret;
}

fn convert_namelist(people: &Vec<Person>) -> Vec<BTreeMap<String, String>> {
    let mut parsed_authors = Vec::<BTreeMap<String, String>>::new();
    for person in people.iter() {
        // ret.parsed_authors.push(format!("{author:?}"));
        parsed_authors.push(convert_person(person));
        // i += 1
    }
    return parsed_authors;
}

fn convert_entry(entry:&Entry) -> MyEntry {
    let name_fields: Vec<&str> = vec![
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
        "translator"        
    ];

    let mut ret = create_entry(entry.entry_type.to_string(), entry.key.clone());

    // store all saved names that biblatex had already parsed
    for name_field in name_fields {
        if entry.fields.contains_key(name_field) {
            let chunk_names = entry.fields.get(name_field).unwrap();
            let names: Vec<Person> = Type::from_chunks(chunk_names).unwrap();
            ret.parsed_names.insert(name_field.to_string(), convert_namelist(&names));
        }
    }

    // store title
    let title = entry.title().unwrap().format_sentence();
    ret.fields.insert("title".to_string(), title);

    // store all the other fields
    insert_converted_fields(entry.fields.clone(), &mut ret.fields);


    // apparently these special cases are no longer needed, I'm still
    // leaving them here for pedagogical reasons
    // match entry.url() {
    //     Ok(url) => ret.fields.insert("url".to_string(), url),
    //     _ => Some("".to_string())
    // };

    // match entry.doi() {
    //     Ok(doi) => ret.fields.insert("doi".to_string(), doi),
    //     _ => Some("".to_string())
    // };

    // let keys: String = entry.fields.keys()
    //                       .map(|key| key.to_string())
    //                       .collect::<Vec<String>>()
    //                       .join(", "); // Join the keys with commas

    return ret;
}


