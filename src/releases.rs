use models::*;
use schema::*;

use caseless;

use diesel::*;
use diesel::pg::PgConnection;

use serde_json::value::Value;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::io::prelude::*;
use std::io::stderr;
use std::process::Command;
use std::str;

use slog::Logger;

use unicode_normalization::UnicodeNormalization;

// needed for case-insensitivity
use diesel::types::VarChar;
sql_function!(lower, lower_t, (x: VarChar) -> VarChar);

pub fn assign_commits(log: &Logger, release_name: &str, previous_release: &str, release_project_id: i32, path: &str) {
    use diesel::expression::dsl::any;
    use diesel::pg::upsert::*;

    // Could take the connection as a parameter, as problably
    // it's already established somewhere...
    let connection = ::establish_connection();

    info!(log, "Assigning commits to release {}", release_name);

    let git_log = Command::new("git")
        .arg("-C")
        .arg(&path)
        .arg("--no-pager")
        .arg("log")
        .arg("--use-mailmap")
        .arg(r#"--format=%H %ae %an"#)
        .arg(&format!("{}...{}", previous_release, release_name))
        .output()
        .expect("failed to execute process");

    let the_release = releases::table
        .filter(releases::version.eq(&release_name))
        .filter(releases::project_id.eq(release_project_id))
        .first::<Release>(&connection)
        .expect("could not find release");

    let commits = str::from_utf8(&git_log.stdout).unwrap()
        .split('\n')
        .filter(|s| !s.is_empty())
        .map(|line| {
            let mut parts = line.splitn(3, ' ');
            let sha_name = parts.next().unwrap();
            let author_email = parts.next().unwrap();
            let author_name = parts.next().unwrap();
            (sha_name, author_email, author_name)
        })
        .collect::<Vec<_>>();

    if commits.is_empty() {
        writeln!(
            stderr(),
            "Could not find commits between {} and {} (maybe the tag is \
            missing?) Skipping.",
            previous_release,
            release_name
        ).unwrap();
        // https://github.com/diesel-rs/diesel/issues/797
        return;
    }

    connection.transaction::<_, Box<Error>, _>(|| {
        let (shas, commits): (Vec<_>, Vec<_>) =
            authors_by_sha(&connection, commits)?
                .into_iter()
                .map(|(sha, author_id)| {
                    (sha, NewCommit {
                        sha: sha,
                        release_id: the_release.id,
                        author_id: author_id,
                    })
                })
                .unzip();

        // Set the release id of any commits that already existed
        // FIXME: In Diesel 0.12 collapse this with the next line to use
        // .on_conflict(sha, do_update().set(commits::release_id.eq(the_release.id)))
        let updated = update(commits::table.filter(commits::sha.eq(any(shas))))
            .set(commits::release_id.eq(the_release.id))
            .execute(&connection)?;

        let inserted = insert(&commits.on_conflict_do_nothing())
            .into(commits::table)
            .execute(&connection)?;

        let total = updated + inserted;
        if total == commits.len() {
            Ok(())
        } else {
            Err(format!("Expected to create or update {} commits, \
                         but only {} were", commits.len(), total).into())
        }
    }).expect("Error saving commits and authors");
}

type Sha<'a> = &'a str;
type Email<'a> = &'a str;
type Name<'a> = &'a str;
type AuthorId = i32;

/// Finds or creates all authors from a git log, and returns the given shas
/// zipped with the id of the author in the database.
fn authors_by_sha<'a>(conn: &PgConnection, git_log: Vec<(Sha<'a>, Email, Name)>)
    -> QueryResult<Vec<(Sha<'a>, AuthorId)>>
{
    let new_authors = git_log.iter().map(|&(_, email, name)| {
        NewAuthor { email: email, name: name }
    }).collect();
    let author_ids = ::authors::find_or_create_all(conn, new_authors)?
        .into_iter()
        .map(|author| ((author.email, author.name), author.id))
        .collect::<HashMap<_, _>>();
    Ok(git_log.into_iter()
        .map(|(sha, email, name)| (sha, author_ids[&(email.into(), name.into())]))
        .collect())
}

pub fn create(conn: &PgConnection, version: &str, project_id: i32) -> Release {
    use schema::releases;

    let new_release = NewRelease {
        version: version,
        project_id: project_id,
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

    let results = releases.filter(project_id.eq(project.id))
        .filter(visible.eq(true))
        .load::<Release>(&connection)
        .expect("Error loading releases");

    results.into_iter()
        .rev()
        .map(|r| Value::String(r.version))
        .collect()
}
