extern crate contributors;

extern crate diesel;

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
    use contributors::schema::releases::dsl::*;
    use contributors::models::Release;
    use contributors::schema::commits::dsl::*;
    use contributors::models::Commit;

    let connection = establish_connection();

    let mut resp = reqwest::get("https://api.github.com/repos/rust-lang/rust/commits").unwrap();

    let response: GitHubResponse = resp.json().unwrap();

    // find the master release so we can assign commits to it
    let master_release = releases.filter(version.eq("master")).first::<Release>(&connection).expect("could not find release");

    for object in response.0 {
        println!("Found commit with sha {}", object.sha);

        // do we have this commit? If so, ignore it.
        match commits.filter(sha.eq(&object.sha)).first::<Commit>(&connection) {
            Ok(commit) => {
                println!("Commit {} already in db, skipping", commit.sha);
                continue;
            },
            Err(_) => {
                println!("Creating commit {} for release {}", object.sha, master_release.version);

                // this commit will be part of master
                contributors::create_commit(&connection, &object.sha, &object.commit.author.name, &object.commit.author.email, &master_release);
            },
        };
    }
}
