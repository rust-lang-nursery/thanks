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
struct GitHubResponse(Vec<Object>);

#[derive(Debug,Deserialize)]
struct Object {
    sha: String,
    commit: Commit,
}

#[derive(Debug,Deserialize)]
struct Commit {
    author: Author,
}

#[derive(Debug,Deserialize)]
struct Author {
    name: String,
    email: String,
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
    {
        use contributors::schema::commits::dsl::*;
        use contributors::models::Commit;
        let first_commit = commits.first::<Commit>(&connection);

        if first_commit.is_ok() {
            panic!("you have commits in here already");
        }
    }

    let mut resp = reqwest::get("https://api.github.com/repos/rust-lang/rust/commits").unwrap();

    let response: GitHubResponse = resp.json().unwrap();

    for object in response.0 {
//        let commit = contributors::create_commit(&connection, &commit.sha, &commit.data.author.name, &commit.data.author.email);

        println!("Saved commit with sha {}", object.sha);
    }
}
