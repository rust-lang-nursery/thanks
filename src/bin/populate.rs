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

    // check that we have no releases
    {
        use contributors::schema::releases::dsl::*;
        use contributors::models::Release;
        let first_release = releases.first::<Release>(&connection);

        if first_release.is_ok() {
            panic!("you have releases in here already");
        }
    }
    
    // check that we have no commits
    {
        use contributors::schema::commits::dsl::*;
        use contributors::models::Commit;
        let first_commit = commits.first::<Commit>(&connection);

        if first_commit.is_ok() {
            panic!("you have commits in here already");
        }
    }

    // get path to git repo
    let path = env::args().nth(1).unwrap();
    println!("Path to rust repo: {}", path);
    
    // create releases
    
    println!("creating first release: 0.1");
    let first_release = contributors::create_release(&connection, "0.1");

    println!("Creating other releases");

    let releases = ["0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "0.10", "0.11.0", "0.12.0", "1.0.0-alpha", "1.0.0-alpha.2", "1.0.0-beta", "1.0.0", "1.1.0", "1.2.0", "1.3.0", "1.4.0", "1.5.0", "1.6.0", "1.7.0", "1.8.0", "1.9.0", "1.10.0", "1.11.0", "1.12.0", "1.12.1", "1.13.0", "1.14.0"];
    
    for release in releases.iter() {
        contributors::create_release(&connection, release);
    }


    // create most commits
    //
    // due to the way git works, this will not create any commits that were backported
    // so we'll do those below
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

        // We tag all commits initially to the first release. Each release will
        // set this properly below.
        contributors::create_commit(&connection, &sha, &author_name, &author_email, &first_release);
    }

    // assign commits to their release
    assign_commits("0.2", "0.1");
    assign_commits("0.3", "0.2");
    assign_commits("0.4", "0.3");
    assign_commits("0.5", "0.4");
    assign_commits("0.6", "0.5");
    assign_commits("0.7", "0.6");
    assign_commits("0.8", "0.7");
    assign_commits("0.9", "0.8");
    assign_commits("0.10", "0.9");
    assign_commits("0.11.0", "0.10");
    assign_commits("0.12.0", "0.11.0");
    assign_commits("1.0.0-alpha", "0.12.0");
    assign_commits("1.0.0-alpha.2", "1.0.0-alpha");
    assign_commits("1.0.0-beta", "1.0.0-alpha.2");
    assign_commits("1.0.0", "1.0.0-beta");
    assign_commits("1.1.0", "1.0.0");
    assign_commits("1.2.0", "1.1.0");
    assign_commits("1.3.0", "1.2.0");
    assign_commits("1.4.0", "1.3.0");
    assign_commits("1.5.0", "1.4.0");
    assign_commits("1.6.0", "1.5.0");
    assign_commits("1.7.0", "1.6.0");
    assign_commits("1.8.0", "1.7.0");
    assign_commits("1.9.0", "1.8.0");
    assign_commits("1.10.0", "1.9.0");
    assign_commits("1.11.0", "1.10.0");
    assign_commits("1.12.0", "1.11.0");
    assign_commits("1.12.1", "1.12.0");
    assign_commits("1.13.0", "1.12.0");
    assign_commits("1.14.0", "1.13.0");

    println!("Done!");
}

fn assign_commits(release_name: &str, previous_release: &str) {
    let connection = establish_connection();
    let path = env::args().nth(1).unwrap();

    println!("Assigning commits to release {}", release_name);

    let git_log = Command::new("git")
        .current_dir(&path)
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
        use contributors::schema::releases::dsl::*;
        use contributors::models::Release;
        use contributors::schema::commits::dsl::*;
        use contributors::models::Commit;

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
                    .current_dir(&path)
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

                contributors::create_commit(&connection, &the_sha, &the_author_name, &the_author_email, &the_release);
            },
        };

    }
}
