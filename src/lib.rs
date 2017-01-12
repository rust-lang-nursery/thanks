#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;

extern crate dotenv;

use diesel::prelude::*;
use diesel::pg::PgConnection;

use dotenv::dotenv;

use std::env;

pub mod schema;
pub mod models;

use self::models::{Commit, NewCommit};
use self::models::{Release, NewRelease};

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
