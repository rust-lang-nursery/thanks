use models::{Commit, NewCommit};
use models::Author;
use models::Release;

use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;

pub fn create<'a>(conn: &PgConnection, sha: &'a str, author: &Author, release: &Release) -> Commit {
    use schema::commits;

    let new_commit = NewCommit {
        sha: sha,
        release_id: release.id,
        author_id: author.id,
    };

    diesel::insert_into(commits::table)
        .values(&new_commit)
        .get_result(conn)
        .expect("Error saving new commit")
}
