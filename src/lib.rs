extern crate caseless;
extern crate git2;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate serde_json;
extern crate unicode_normalization;

mod mailmap;

pub use mailmap::Mailmap;
use serde_json::{Map, Value};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

pub fn scores(repo: &git2::Repository, mailmap: &Mailmap) -> Vec<Value> {
    let mut data = HashMap::new();

    let mut walker = repo.revwalk().unwrap();
    walker.push_head().unwrap();

    for oid in walker {
        let oid = oid.unwrap();
        let commit = repo.find_commit(oid).unwrap();
        let signature = commit.author();
        let name = signature.name().unwrap();
        let email = signature.email().unwrap();
        let name = mailmap.map(name, email).0;

        let entry = data.entry(name).or_insert(0);
        *entry += 1;
    }

    let mut scores: Vec<_> = data.into_iter().collect();
    scores.sort_by(|a, b| b.1.cmp(&a.1));

    // these variables are used to calculate the ranking
    let mut rank = 0; // incremented every time
    let mut last_rank = 0; // the current rank
    let mut last_score = 0; // the previous entry's score

    scores.into_iter().map(|(author, score)| {
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
    }).collect()
}

pub fn names(release_name: &str, previous_release: &str, repo: &git2::Repository, mailmap: &Mailmap) -> Vec<Value> {
    // fetch info
    let mut data = HashSet::new();

    let mut walker = repo.revwalk().unwrap();
    walker.push_range(&format!("{}..{}", previous_release, release_name)).unwrap();

    for oid in walker {
        let oid = oid.unwrap();
        let commit = repo.find_commit(oid).unwrap();
        let signature = commit.author();
        let name = signature.name().unwrap();
        let email = signature.email().unwrap();
        data.insert(mailmap.map(name, email).0);
    }

    let mut names: Vec<String> = data.into_iter().collect();
    inaccurate_sort(&mut names);
    names.into_iter().map(|v| Value::from(v)).collect()
}

// TODO: switch this out for an implementation of the Unicode Collation Algorithm
fn inaccurate_sort(strings: &mut Vec<String>) {
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
