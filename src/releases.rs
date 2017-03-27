extern crate git2;

use models::*;
use schema::*;

use caseless;

use diesel::*;
use diesel::pg::PgConnection;

use authors::AuthorStore;

use serde_json::value::Value;

use semver::Version;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::io::stderr;
use std::str;

use slog::Logger;

use unicode_normalization::UnicodeNormalization;
use releases::git2::Repository;
use releases::git2::Oid;

// needed for case-insensitivity
use diesel::types::VarChar;
sql_function!(lower, lower_t, (x: VarChar) -> VarChar);

impl Release {
    /// provide a semver-compatible version
    ///
    /// rust's older versions were missing a minor version and so are not semver-compatible
    fn semver_version(&self) -> Version {
        Version::parse(&self.version).unwrap_or_else(|_| {
            let v = format!("{}.0", self.version);
            Version::parse(&v).unwrap()
        })
    }
}

use std::io::prelude::*;

pub fn assign_commits(log: &Logger, repo: &Repository, cache: &mut AuthorStore, release_name: &str, commits: Vec<Oid>, release_project_id: i32) {
    use diesel::pg::upsert::*;

    // Could take the connection as a parameter, as problably
    // it's already established somewhere...
    let connection = ::establish_connection();

    info!(log, "Assigning commits to release {}", release_name);


    let the_release = releases::table
        .filter(releases::version.eq(&release_name))
        .filter(releases::project_id.eq(release_project_id))
        .first::<Release>(&connection)
        .expect("could not find release");

    let temp_commits = commits.into_iter().map(|id| {
        let commit = repo.find_commit(id).unwrap();
        let author = commit.author().to_owned();
        (commit, author)
    }).collect::<Vec<_>>();
    let mut parsed_commits = Vec::new();

    for &(ref commit, ref author) in temp_commits.iter() {
        let (mapped_name, mapped_email) = cache.get_mailmap().map(author.name().unwrap(), author.email().unwrap());
        parsed_commits.push((format!("{}", commit.id()), mapped_name, mapped_email));
    }

    if parsed_commits.is_empty() {
        writeln!(
            stderr(),
            "Could not find commits for {} (maybe the tag is \
            missing?) Skipping.",
            release_name
        ).unwrap();
        // https://github.com/diesel-rs/diesel/issues/797
        return;
    }

    connection.transaction::<_, Box<Error>, _>(|| {
        let by_sha = authors_by_sha(cache, parsed_commits);
        let commits: Vec<_> = {
            by_sha
                .iter()
                .map(|&(ref sha, author_id)| {
                    NewCommit {
                        sha: sha.as_str(),
                        release_id: the_release.id,
                        author_id: author_id,
                    }
                }).collect()
        };


        // Set the release id of any commits that already existed
        let inserted = insert(&commits.on_conflict(commits::sha, do_update().set(commits::release_id.eq(the_release.id))))
            .into(commits::table)
            .execute(&connection)?;

        if inserted == commits.len() {
            Ok(())
        } else {
            Err(format!("Expected to create or update {} commits, \
                         but only {} were", commits.len(), inserted).into())
        }
    }).expect("Error saving commits and authors");
}

pub fn get_first_commits(repo: &Repository, release_name: &str) -> Vec<Oid> {
    let mut walk = repo.revwalk().unwrap();
    walk.push(repo.revparse(release_name).unwrap().from().unwrap().id()).unwrap();
    walk.into_iter().map(|id| id.unwrap()).collect()
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

type AuthorId = i32;

/// Finds or creates all authors from a git log, and returns the given shas
/// zipped with the id of the author in the database.
fn authors_by_sha<'a>(cache: &mut AuthorStore<'a>, git_log: Vec<(String, String, String)>)
    -> Vec<(String, AuthorId)>
{
    let new_authors = git_log.iter().map(|&(_, ref name, ref email)| {
        NewAuthor { email: email.as_str(), name: name.as_str() }
    }).collect();
    let author_ids = cache.find_or_create_all(new_authors)
        .into_iter()
        .map(|author| ((author.name, author.email), author.id))
        .collect::<HashMap<_, _>>();
    git_log.iter()
        .map(|&(ref sha, ref name, ref email)| {
            (sha.clone(), author_ids[&(name.clone(), email.clone())])
        })
        .collect()
}

pub fn create(conn: &PgConnection, version: &str, project_id: i32, visible: bool) -> Release {
    use schema::releases;

    let new_release = NewRelease {
        version: version,
        project_id: project_id,
        visible: visible,
    };

    insert(&new_release).into(releases::table)
        .get_result(conn)
        .expect("Error saving new release")
}

pub fn contributors(project: &str, release_name: &str) -> Option<Vec<Value>> {
    use schema::releases::dsl::*;
    use schema::commits::dsl::*;
    use models::Release;

    let connection = ::establish_connection();

    let project = {
        use schema::projects::dsl::*;

        match projects.filter(lower(name).eq(lower(project)))
            .first::<Project>(&connection) {
                Ok(p) => p,
                Err(_) => {
                    return None;
                }
        }
    };

    let release: Release = match releases
        .filter(version.eq(release_name))
        .filter(project_id.eq(project.id))
        .first(&connection) {
            Ok(release) => release,
                Err(_) => {
                    return None;
                },
        };

    // it'd be better to do this in the db
    // but Postgres doesn't do Unicode collation correctly on OSX
    // http://postgresql.nabble.com/Collate-order-on-Mac-OS-X-text-with-diacritics-in-UTF-8-td1912473.html
    use schema::authors;
    let mut names: Vec<String> = authors::table.inner_join(commits).filter(release_id.eq(release.id))
        .filter(authors::visible.eq(true)).select(authors::name).distinct().load(&connection).unwrap();

    inaccurate_sort(&mut names);

    Some(names.into_iter().map(Value::String).collect())
}

// TODO: switch this out for an implementation of the Unicode Collation Algorithm
pub fn inaccurate_sort(strings: &mut Vec<String>) {
    strings.sort_by(|a, b| str_cmp(&a, &b));
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

/// returns all releases
///
/// sorted in semver order
pub fn all() -> Vec<Value> {
    use schema::releases::dsl::*;
    use models::Release;
    use models::Project;

    let connection = ::establish_connection();

    let project = {
        use schema::projects::dsl::*;
        projects.filter(name.eq("Rust"))
            .first::<Project>(&connection)
        .expect("Error finding the Rust project")
    };

    let mut results = releases.filter(project_id.eq(project.id))
        .filter(visible.eq(true))
        .load::<Release>(&connection)
        .expect("Error loading releases");

    // sort the versions
    //
    // first we need to remove master as it is not a valid semver version, and
    // master should be at the top anyway
    let master = match results.iter().position(|r| r.version == "master") {
        Some(i) => results.remove(i),
        None => panic!("master release not found"),
    };

    // next up, sort by semver version
    results.sort_by(|a, b| {
        a.semver_version().cmp(&b.semver_version())
    });

    // finally, push master/all-time back at the top
    results.push(master);

    results.into_iter()
        .rev()
        .map(|r| Value::String(r.version))
        .collect()
}
