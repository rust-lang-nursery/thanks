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
use std::process::Command;

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

    let path = env::args().nth(1).unwrap();
    println!("Path to rust repo: {}", path);

    let git_log = Command::new("git")
        .current_dir(path)
        .arg("--no-pager")
        .arg("log")
        .arg(r#"--format=%H %ae %an"#)
        .arg("master")
        .output()
        .expect("failed to execute process");

    let log = git_log.stdout;
    let log = String::from_utf8(log).unwrap();

    for log_line in log.split('\n') {
        // there is a last, blank line
        if log_line == "" {
            continue;
        }

        let mut split = log_line.splitn(3, ' ');

        let sha = split.next().unwrap();
        let author_email = split.next().unwrap();
        let author_name = split.next().unwrap();

        println!("Creating commit: {}", sha);
        contributors::create_commit(&connection, &sha, &author_name, &author_email);
    }

    println!("Done!");
}
