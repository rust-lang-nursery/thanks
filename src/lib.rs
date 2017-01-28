#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;

extern crate dotenv;

use diesel::prelude::*;
use diesel::pg::PgConnection;

use dotenv::dotenv;

extern crate caseless;
extern crate unicode_normalization;

use std::collections::BTreeMap;
use std::env;
use std::cmp::Ordering;
use std::process::Command;

extern crate serde_json;

#[macro_use]
extern crate slog;
extern crate slog_term;

pub mod schema;
pub mod models;

use self::models::{Project, NewProject};
use self::models::{Commit, NewCommit};
use self::models::{Release, NewRelease};

use unicode_normalization::UnicodeNormalization;

use serde_json::value::Value;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

pub fn create_commit<'a>(conn: &PgConnection, sha: &'a str, author_name: &'a str, author_email: &'a str, release: &Release) -> Commit {
    use schema::commits;

    let new_commit = NewCommit {
        sha: sha,
        release_id: release.id,
        author_name: author_name,
        author_email: author_email,
    };

    diesel::insert(&new_commit).into(commits::table)
        .get_result(conn)
        .expect("Error saving new commit")
}

pub fn create_project(conn: &PgConnection, name: &str, url_path: &str, github_name: &str) -> Project {
    use schema::projects;

    let new_project = NewProject {
        name: name,
        url_path: url_path,
        github_name: github_name
    };

    diesel::insert(&new_project).into(projects::table)
        .get_result(conn)
        .expect("Error saving new project")
}


pub fn create_release(conn: &PgConnection, version: &str, project_id: i32) -> Release {
    use schema::releases;

    let new_release = NewRelease {
        version: version,
        project_id: project_id,
    };

    diesel::insert(&new_release).into(releases::table)
        .get_result(conn)
        .expect("Error saving new release")
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

// TODO: switch this out for an implementation of the Unicode Collation Algorithm
pub fn inaccurate_sort(strings: &mut Vec<String>) {
    strings.sort_by(|a, b| str_cmp(&a, &b));
}

pub fn assign_commits(log: &slog::Logger, release_name: &str, previous_release: &str, release_project_id: i32, path: &str) {
    // Could take the connection as a parameter, as problably
    // it's already established somewhere...
    let connection = establish_connection();

    info!(log, "Assigning commits to release {}", release_name);

    let git_log = Command::new("git")
        .arg("-C")
        .arg(&path)
        .arg("--no-pager")
        .arg("log")
        .arg("--use-mailmap")
        .arg(r#"--format=%H"#)
        .arg(&format!("{}...{}", previous_release, release_name))
        .output()
        .expect("failed to execute process");

    let git_log = git_log.stdout;
    let git_log = String::from_utf8(git_log).unwrap();

    for sha_name in git_log.split('\n') {
        // there is a last, blank line
        if sha_name == "" {
            continue;
        }

        info!(log, "Assigning commit {} to release {}", sha_name, release_name);

        use schema::releases::dsl::*;
        use models::Release;
        use schema::commits::dsl::*;
        use models::Commit;

        let the_release = releases
            .filter(version.eq(&release_name))
            .filter(project_id.eq(release_project_id))
            .first::<Release>(&connection)
            .expect("could not find release");

        // did we make this commit earlier? If so, update it. If not, create it
        match commits.filter(sha.eq(&sha_name)).first::<Commit>(&connection) {
            Ok(the_commit) => {
                diesel::update(commits.find(the_commit.id))
                    .set(release_id.eq(the_release.id))
                    .get_result::<Commit>(&connection)
                    .expect(&format!("Unable to update commit {}", the_commit.id));
            },
            Err(_) => {
                let git_log = Command::new("git")
                    .arg("-C")
                    .arg(&path)
                    .arg("--no-pager")
                    .arg("show")
                    .arg(r#"--format=%H %ae %an"#)
                    .arg(&sha_name)
                    .output()
                    .expect("failed to execute process");

                let git_log = git_log.stdout;
                let git_log = String::from_utf8(git_log).unwrap();

                let log_line = git_log.split('\n').nth(0).unwrap();

                let mut split = log_line.splitn(3, ' ');

                let the_sha = split.next().unwrap();
                let the_author_email = split.next().unwrap();
                let the_author_name = split.next().unwrap();

                info!(log, "Creating commit {} for release {}", the_sha, the_release.version);

                create_commit(&connection, &the_sha, &the_author_name, &the_author_email, &the_release);
            },
        };
    }
}

pub fn releases() -> Vec<Value> {
    use schema::releases::dsl::*;
    use models::Release;

    let connection = establish_connection();
    let results = releases.filter(version.ne("master"))
        .load::<Release>(&connection)
        .expect("Error loading releases");

    results.into_iter()
        .rev()
        .map(|r| Value::String(r.version))
        .collect()
}

pub fn scores() -> Vec<Value> {
    use schema::commits::dsl::*;
    use diesel::expression::dsl::sql;
    use diesel::types::BigInt;

    let connection = establish_connection();

    let scores: Vec<_> =
        commits
        .select((author_name, sql::<BigInt>("COUNT(author_name) AS author_count")))
        .group_by(author_name)
        .order(sql::<BigInt>("author_count").desc())
        .load(&connection)
        .unwrap();

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

        let mut json_score: BTreeMap<String, Value> = BTreeMap::new();

        // we use last_rank here so that we get duplicate ranks for people
        // with the same number of commits
        json_score.insert("rank".to_string(), Value::I64(last_rank));

        json_score.insert("author".to_string(), Value::String(author));
        json_score.insert("commits".to_string(), Value::I64(score));

        Value::Object(json_score)
    }).collect()
}

pub fn names(release_name: &str) -> Option<Vec<Value>> {
    use schema::releases::dsl::*;
    use schema::commits::dsl::*;
    use models::Release;
    use models::Commit;

    let connection = establish_connection();

    let release: Release = match releases.filter(version.eq(release_name))
        .first(&connection) {
            Ok(release) => release,
                Err(_) => {
                    return None;
                },
        };

    // it'd be better to do this in the db
    // but Postgres doesn't do Unicode collation correctly on OSX
    // http://postgresql.nabble.com/Collate-order-on-Mac-OS-X-text-with-diacritics-in-UTF-8-td1912473.html
    let mut names: Vec<String> = Commit::belonging_to(&release)
        .select(author_name).distinct().load(&connection).unwrap();

    inaccurate_sort(&mut names);

    Some(names.into_iter().map(Value::String).collect())
}
