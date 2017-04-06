use caseless;
use futures::BoxFuture;
use sparkles::{Request, Response, ResponseBuilder, Status, Error};
use REPOSITORY;
use serde_json::Value;
use std::collections::HashSet;
use std::cmp::Ordering;
use regex::Captures;
use MAILMAP;

/// The handler for /
pub fn root(_: Request) -> BoxFuture<Response, Error> {
    let mut res = ResponseBuilder::new();
    res.with_template("index".to_string());

    let releases = vec![
        "1.16.0",
        "1.15.1",
        "1.15.0",
    ];

    res.data.insert("releases".to_string(),
                Value::from(releases));

    res.with_status(Status::Ok);

    res.to_response().into_future()
}

/// for /{{project}}/{{release}}
pub fn release(_: &Request, cap: Captures) -> BoxFuture<Response, Error> {
    let mut res = ResponseBuilder::new();
    res.with_template("release".to_string());

    let project = cap.get(1).unwrap();
    let project = project.as_str();

    let release_name = cap.get(2).unwrap();
    let release_name = release_name.as_str();

    res.data.insert("release".to_string(), Value::String(release_name.to_string()));

    // fetch info
    let repo = REPOSITORY.lock().unwrap();

    let mut data = HashSet::new();

    let mut walker = repo.revwalk().unwrap();
    walker.push_range("1.15.0..1.16.0").unwrap();

    for oid in walker {
        let oid = oid.unwrap();
        let commit = repo.find_commit(oid).unwrap();
        let signature = commit.author();
        let name = signature.name().unwrap();
        let email = signature.email().unwrap();
        data.insert(MAILMAP.map(name, email).0);
    }

    let mut names: Vec<String> = data.into_iter().collect();
    inaccurate_sort(&mut names);
    let names: Vec<_> = names.into_iter().map(|v| Value::from(v)).collect();

    res.data.insert("count".to_string(), Value::Number((names.len() as u64).into()));
    res.data.insert("names".to_string(), Value::Array(names));
    res.with_status(Status::Ok);

    res.to_response().into_future()
}

// TODO: switch this out for an implementation of the Unicode Collation Algorithm
pub fn inaccurate_sort(strings: &mut Vec<String>) {
    strings.sort_by(|a, b| str_cmp(&a, &b));
}

fn str_cmp(a_raw: &str, b_raw: &str) -> Ordering {
    use unicode_normalization::UnicodeNormalization;
    let a: Vec<char> = a_raw.nfkd().filter(|&c| (c as u32) < 0x300 || (c as u32) > 0x36f).collect();
    let b: Vec<char> = b_raw.nfkd().filter(|&c| (c as u32) < 0x300 || (c as u32) > 0x36f).collect();

    for (&a_char, &b_char) in a.iter().zip(b.iter()) {
        match char_cmp(a_char, b_char) {
            Ordering::Less => return Ordering::Less,
            Ordering::Greater => return Ordering::Greater,
            Ordering::Equal => {}
        }
    }

    if a.len() < b.len() {
        Ordering::Less
    } else if a.len() > b.len() {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

fn char_cmp(a_char: char, b_char: char) -> Ordering {
    let a = caseless::default_case_fold_str(&a_char.to_string());
    let b = caseless::default_case_fold_str(&b_char.to_string());

    let first_char = a.chars().nth(0).unwrap_or('{');

    let order = if a == b && a.len() == 1 && 'a' <= first_char && first_char <= 'z' {
        if a_char > b_char {
            Ordering::Less
        } else if a_char < b_char {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    } else {
        a.cmp(&b)
    };

    order
}
