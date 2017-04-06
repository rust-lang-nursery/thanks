use caseless;
use futures::BoxFuture;
use sparkles::{Request, Response, ResponseBuilder, Status, Error};
use REPOSITORY;
use serde_json::{Value, Map};
use std::collections::{HashSet, HashMap};
use std::cmp::Ordering;
use regex::Captures;
use MAILMAP;
use RELEASES;

/// The handler for /
pub fn root(_: Request) -> BoxFuture<Response, Error> {
    let mut res = ResponseBuilder::new();
    res.with_template("index".to_string());

    let mut releases = vec!["master"];
    releases.extend(RELEASES.into_iter().map(|&(a, b, c)| a));

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

    let (release_name, previous, notes) = if release_name == "master" {
        ("HEAD", RELEASES[0].1, "https://github.com/rust-lang/rust/blob/master/RELEASES.md")
    } else {
        *RELEASES.iter().find(|&&(r, p, n)| r == release_name).unwrap()
    };

    // fetch info
    let repo = REPOSITORY.lock().unwrap();

    let mut data = HashSet::new();

    let mut walker = repo.revwalk().unwrap();
    walker.push_range(&format!("{}..{}", previous, release_name)).unwrap();

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
    res.data.insert("link".to_string(), Value::String(notes.to_string()));
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

pub fn about(_: Request) -> BoxFuture<Response, Error> {
    let mut res = ResponseBuilder::new();
    res.with_template("about".to_string());

    res.with_status(Status::Ok);

    res.to_response().into_future()
}

pub fn all_time(_: Request) -> BoxFuture<Response, Error> {
    let mut res = ResponseBuilder::new();
    res.with_template("all-time".to_string());

    let repo = REPOSITORY.lock().unwrap();

    let mut data = HashMap::new();

    let mut walker = repo.revwalk().unwrap();
    walker.push_head().unwrap();

    for oid in walker {
        let oid = oid.unwrap();
        let commit = repo.find_commit(oid).unwrap();
        let signature = commit.author();
        let name = signature.name().unwrap();
        let email = signature.email().unwrap();
        let name = MAILMAP.map(name, email).0;

        let entry = data.entry(name).or_insert(0);
        *entry += 1;
    }

    let mut scores: Vec<_> = data.into_iter().collect();
    scores.sort_by(|a, b| b.1.cmp(&a.1));

    // these variables are used to calculate the ranking
    let mut rank = 0; // incremented every time
    let mut last_rank = 0; // the current rank
    let mut last_score = 0; // the previous entry's score

    let scores: Vec<_> = scores.into_iter().map(|(author, score)| {
        // we always increment the ranking
        rank += 1;

        // if we've hit a different score...
        if last_score != score {

            // then we need to save these values for the future iteration
            last_rank = rank;
            last_score = score;
        }

        let mut json_score: Map<String, Value> = Map::new();

        // we use last_rank here so that we get duplicate ranks for people
        // with the same number of commits
        json_score.insert("rank".to_string(), Value::Number(last_rank.into()));

        json_score.insert("author".to_string(), Value::String(author));
        json_score.insert("commits".to_string(), Value::Number(score.into()));

        Value::Object(json_score)
    }).collect();

    res.data.insert("release".to_string(),
                Value::String(String::from("all-time")));
    res.data.insert("count".to_string(), Value::Number((scores.len() as u64).into()));
    res.data.insert("scores".to_string(), Value::Array(scores));

    res.with_status(Status::Ok);

    res.to_response().into_future()
}