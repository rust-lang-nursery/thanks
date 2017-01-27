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

#[macro_use]
extern crate slog;
extern crate slog_term;

use slog::DrainExt;

use diesel::prelude::*;

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

fn main() {
    let log = slog::Logger::root(slog_term::streamer().full().build().fuse(), o!("version" => env!("CARGO_PKG_VERSION")));

    use contributors::schema::releases::dsl::*;
    use contributors::models::Release;
    use contributors::schema::commits::dsl::*;
    use contributors::models::Commit;

    let connection = contributors::establish_connection();

    let mut resp = reqwest::get("https://api.github.com/repos/rust-lang/rust/commits").unwrap();

    let response: GitHubResponse = resp.json().unwrap();

    // find the master release so we can assign commits to it
    let master_release = releases.filter(version.eq("master")).first::<Release>(&connection).expect("could not find release");

    for object in response.0 {
        info!(log, "Found commit with sha {}", object.sha);

        // do we have this commit? If so, ignore it.
        match commits.filter(sha.eq(&object.sha)).first::<Commit>(&connection) {
            Ok(commit) => {
                info!(log, "Commit {} already in db, skipping", commit.sha);
                continue;
            },
            Err(_) => {
                info!(log, "Creating commit {} for release {}", object.sha, master_release.version);

                // this commit will be part of master
                contributors::create_commit(&connection, &object.sha, &object.commit.author.name, &object.commit.author.email, &master_release);
            },
        };
    }
}
