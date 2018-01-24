extern crate git2;

extern crate time;

use caseless;

use serde_json::value::Value;

use semver::Version;

use std::cmp::Ordering;
use std::str;

use unicode_normalization::UnicodeNormalization;

use releases::git2::Repository;
use releases::git2::Oid;

// needed for case-insensitivity
//use diesel::types::VarChar;
//sql_function!(lower, lower_t, (x: VarChar) -> VarChar);

/// provide a semver-compatible version
///
/// rust's older versions were missing a minor version and so are not semver-compatible
fn semver_version(version: &str) -> Version {
    Version::parse(version).unwrap_or_else(|_| {
        let v = format!("{}.0", version);
        Version::parse(&v).unwrap()
    })
}

/// `get_first_commits`
pub fn get_first_commits(repo: &Repository, release_name: &str) -> Option<Vec<Oid>> {
    let mut walk = match repo.revwalk() {
        Ok(v) => v,
        Err(e) => {
            println!("no rev walk: {}", e);
            return None;
        },
    };

    walk.push(repo.revparse(release_name).unwrap().from().unwrap().id()).unwrap();
    Some(walk.into_iter().map(|id| id.unwrap()).collect())
}


// libgit2 currently doesn't support the symmetric difference (triple dot or 'A...B') notation.
// We replicate it using the union of 'A..B' and 'B..A'
pub fn get_commits(repo: &Repository, release_name: &str, previous_release: &str) -> Vec<Oid> {
    let mut walk_1 = repo.revwalk().unwrap();
    walk_1.push_range(format!("{}..{}", previous_release, release_name).as_str()).unwrap();

    let mut walk_2 = repo.revwalk().unwrap();
    walk_2.push_range(format!("{}..{}", release_name, previous_release).as_str()).unwrap();

    walk_1.into_iter().map(|id| id.unwrap()).chain(walk_2.into_iter().map(|id| id.unwrap())).collect()
}


pub fn contributors(tags: &[Value], repo_path: &str, release_name: &str) -> Option<Vec<Value>> {
    let names = walk_release(tags, repo_path, release_name);

    Some(names.into_iter().map(Value::String).collect())
}

/// `walk_release` walks a tag, collecting author names.
fn walk_release(tags: &[Value], repo_path: &str, release_name: &str) -> Vec<String>{
    let repo = match Repository::open(repo_path) {
        Ok(v) => v,
        Err(e) => panic!("failed to open: {}", e),
    };

    let mut walk = match repo.revwalk() {
        Ok(v) => v,
        Err(e) => panic!("failed getting revwalk: {}", e),
    };

    // Set the range of commits to walk by setting the bounding tag refs.
    // First set the tag we're interested in, at which we start walking.
    let refs_tags_prefix = "refs/tags/";

    let tag_ref = match release_name {
        "master" => "refs/heads/master".to_string(),
        _ => [refs_tags_prefix, release_name].concat(),
    };

    match walk.push_ref(&tag_ref) {
        Ok(()) => (),
        Err(e) => panic!("failed pushing ref onto revwalk: {}", e),
    };

    // Set the tag at which we want to stop walking.
    if let Some(prev_tag) = find_previous_tag_ref(tags, release_name) {
        let prev_tag_ref = [refs_tags_prefix, prev_tag.as_str().unwrap()].concat().to_string();
        match walk.hide_ref(&prev_tag_ref) {
            Ok(()) => (),
            Err(e) => panic!("failed hiding ref in revwalk: {}", e),
        };
    }

    // Walk the commit graph and collect the authors.
    let mut authors: Vec<String> = vec!();
    for res in walk {
        let oid = match res {
            Ok(v) => v,
            Err(e) => panic!("failed getting object walked on: {}", e),
        };

        let commit = match repo.find_commit(oid) {
            Ok(v) => v,
            Err(e) => panic!("walked commit oid is missing or not a commit: {}", e),
        };

        let author = commit.author().to_owned();
        
        let author_name = match author.name() {
            Some(v) => v.to_owned(),
            None => panic!("failed getting author name"),
        };
        
        authors.push(author_name);
    }

    authors.sort_by(|a, b| str_cmp(a, b));
    authors.dedup();
    authors
}

/// `find_previous_tag_ref` finds the tag preceding the specified tag.
/// This is used to find a range of commits for a tag.
fn find_previous_tag_ref(tags: &[Value], tag_ref: &str) -> Option<Value> {
    let mut found_tag = false;

    for t in tags {
        if found_tag {
            return Some(t.clone());
        }
        if t == tag_ref {
            found_tag = true;
        }
    }

    None
}

// TODO: switch this out for an implementation of the Unicode Collation Algorithm
pub fn inaccurate_sort(strings: &mut Vec<String>) {
    strings.sort_by(|a, b| str_cmp(a, b));
}

fn str_cmp(a_raw: &str, b_raw: &str) -> Ordering {
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

    if a == b && a.len() == 1 && 'a' <= first_char && first_char <= 'z' {
        if a_char > b_char {
            Ordering::Less
        } else if a_char < b_char {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    } else {
        a.cmp(&b)
    }
}

/// `all` returns all the tags found in the git repository, 
/// sorted in semver order.
pub fn all(repo_path: &str) -> Vec<Value> {
    let repo = match Repository::open(repo_path) {
        Ok(v) => v,
        Err(e) => panic!("failed opening git repository: {}", e),
    };

    let tag_names_array = match repo.tag_names(None) {
        Ok(v) => v,
        Err(e) => panic!("failed retrieving tag names from git: {}", e),
    };

    // Extract tag names from Options.
    let tag_names: Vec<&str> = tag_names_array.into_iter().filter_map(|o| o).collect();
    
    // Split tag names into versions and "release-*"s - 
    // the latter are not valid semvers, which hinders sorting.
    // Get those without "release-".
    let mut version_names: Vec<&str> = tag_names.clone()
        .into_iter()
        .filter(|t| ! t.starts_with("release-")).collect();

    // Get those starting with "release-".
    let release_names: Vec<&str> = tag_names.clone()
        .into_iter()
        .filter(|t| t.starts_with("release-")).collect();

    // This sorts a Vec<&str> of semvers.
    version_names.sort_by(|a, b| {
        semver_version(a).cmp(&semver_version(b))
    });
    
    // This sets whether the early "release-*" tags are included.
    let include_early_releases = true;

    let all_tags = if include_early_releases {
        [&release_names[..], &version_names[..], &["master"]].concat()
    } else {
        version_names.clone()
    };

    all_tags.into_iter()
        .rev()
        .map(|r| Value::String(r.to_string()))
        .collect()
}

