extern crate contributors;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;

extern crate dotenv;

extern crate futures;

extern crate handlebars;

extern crate reqwest;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;

use std::env;

#[derive(Debug,Deserialize)]
struct GitHubResponse {
    url: String,
    total_commits: u32,
    commits: Vec<Commit>
}

#[derive(Debug,Deserialize)]
struct Commit {
    sha: String,
    #[serde(rename = "commit")]
    data: CommitData,
}

#[derive(Debug,Deserialize)]
struct CommitData {
    author: Author,
}

#[derive(Debug,Deserialize)]
struct Author {
    name: String,
    email: String,
    date: String,
}

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

fn main() {
    let connection = establish_connection();

    let mut resp = reqwest::get("https://api.github.com/repos/rust-lang/rust/compare/1.13.0...1.14.0").unwrap();

    let response: GitHubResponse = resp.json().unwrap();

    for commit in response.commits {
        let commit = contributors::create_commit(&connection, &commit.sha, &commit.data.author.name, &commit.data.author.email);

        println!("\nSaved commit with sha {}", commit.sha);
    }
}
