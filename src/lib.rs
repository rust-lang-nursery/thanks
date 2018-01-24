#[macro_use]
extern crate diesel;

#[macro_use]
extern crate lazy_static;

extern crate dotenv;

extern crate semver;
extern crate regex;

use diesel::prelude::*;
use diesel::pg::PgConnection;

use dotenv::dotenv;

extern crate caseless;
extern crate unicode_normalization;
extern crate git2;

use git2::Repository;

use std::env;

use std::collections::HashMap;

extern crate serde_json;

use serde_json::Map;

extern crate slog;
extern crate slog_term;
extern crate slog_async;

pub mod schema;
pub mod models;

pub mod projects;
pub mod releases;
pub mod mailmap;

use serde_json::value::Value;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

pub fn scores(repo_path: &str) -> Vec<Value> {
    let repo = match Repository::open(repo_path) {
        Ok(v) => v,
        Err(e) => panic!("failed to open: {}", e),
    };

    let mut walk = match repo.revwalk() {
        Ok(v) => v,
        Err(e) => panic!("failed getting revwalk: {}", e),
    };

    match walk.push_head() {
        Ok(()) => (),
        Err(e) => panic!("failed pushing head onto revwalk: {}", e),
    };

    // Walk the commit graph and collect the authors.
    let mut auth_count: HashMap<String, u64> = HashMap::new();
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
        
        let mut author_name = match author.name() {
            Some(v) => v.to_owned(),
            None => panic!("failed getting author name"),
        };
        
        let counter = auth_count.entry(author_name).or_insert(0);
        *counter += 1;
    }

    // Convert the dictionary to a vec of tuples: String, u64.
    let mut scores: Vec<(String, u64)> = vec!();

    for (k, v) in &auth_count {
        scores.push((k.to_owned(), v.to_owned()));
    }

    scores.sort_by(|&(_, ref b), &(_, ref d)| d.cmp(b));

    // End of walk

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

/// `in_maintenace` checks the db to see if we are in maintenance mode.
pub fn in_maintenance() -> bool {
    use models::Maintenance;
    use schema::maintenances::dsl::*;

    let connection = establish_connection();

    let model = maintenances.find(1)
            .load::<Maintenance>(&connection)
            .expect("Error loading maintenance model").remove(0);

    model.enabled
}
