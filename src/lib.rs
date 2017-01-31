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

extern crate serde_json;

#[macro_use]
extern crate slog;
extern crate slog_term;

pub mod schema;
pub mod models;

pub mod projects;
pub mod releases;
pub mod commits;

use serde_json::value::Value;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
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
