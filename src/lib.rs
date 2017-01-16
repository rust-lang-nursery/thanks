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

use std::env;
use std::cmp::Ordering;
use std::process::Command;

pub mod schema;
pub mod models;

use self::models::{Commit, NewCommit};
use self::models::{Release, NewRelease};

use unicode_normalization::UnicodeNormalization;

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


pub fn create_release(conn: &PgConnection, version: &str) -> Release {
    use schema::releases;

    let new_release = NewRelease {
        version: version,
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

pub fn assign_commits(release_name: &str, previous_release: &str, path: &str) {
    let connection =    establish_connection();

    println!("Assigning commits to release {}", release_name);

    let git_log = Command::new("git")
        .arg("-C")
        .arg(&path)
        .arg("--no-pager")
        .arg("log")
        .arg(r#"--format=%H"#)
        .arg(&format!("{}...{}", previous_release, release_name))
        .output()
        .expect("failed to execute process");

    let log = git_log.stdout;
    let log = String::from_utf8(log).unwrap();

    for sha_name in log.split('\n') {
        // there is a last, blank line
        if sha_name == "" {
            continue;
        }

        println!("Assigning commit {} to release {}", sha_name, release_name);

        //contributors::create_commit(&connection, &sha, &author_name, &author_email, &first_release);
        use schema::releases::dsl::*;
        use models::Release;
        use schema::commits::dsl::*;
        use models::Commit;

        let the_release = releases.filter(version.eq(&release_name)).first::<Release>(&connection).expect("could not find release");

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

                let log = git_log.stdout;
                let log = String::from_utf8(log).unwrap();

                let log_line = log.split('\n').nth(0).unwrap();

                let mut split = log_line.splitn(3, ' ');

                let the_sha = split.next().unwrap();
                let the_author_email = split.next().unwrap();
                let the_author_name = split.next().unwrap();

                println!("Creating commit {} for release {}", the_sha, the_release.version);

                create_commit(&connection, &the_sha, &the_author_name, &the_author_email, &the_release);
            },
        };

    }
}
