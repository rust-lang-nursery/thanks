extern crate contributors;

extern crate diesel;

extern crate dotenv;

extern crate futures;

extern crate handlebars;

extern crate reqwest;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate slog;
extern crate slog_term;

extern crate clap;

use diesel::prelude::*;
use clap::{App, Arg};
use slog::DrainExt;

use std::process::Command;

fn main() {
    let matches = App::new("populate")
        .about("initialize the database")
        .arg(Arg::with_name("filepath")
            .short("p")
            .long("path")
            .help("filepath of the rust source code")
            .takes_value(true)
            .required(true))
        .get_matches();

    let log = slog::Logger::root(slog_term::streamer().full().build().fuse(), o!("version" => env!("CARGO_PKG_VERSION")));

    let connection = contributors::establish_connection();

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
    let path = matches.value_of("filepath").unwrap();

    // create releases

    // first, the unrelased commits on master
    contributors::create_release(&connection, "master");

    // then let's get to the real releases:
    info!(log, "creating first release: 0.1");
    let first_release = contributors::create_release(&connection, "0.1");

    info!(log, "Creating other releases");

    let releases = ["0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "0.10", "0.11.0", "0.12.0", "1.0.0-alpha", "1.0.0-alpha.2", "1.0.0-beta", "1.0.0", "1.1.0", "1.2.0", "1.3.0", "1.4.0", "1.5.0", "1.6.0", "1.7.0", "1.8.0", "1.9.0", "1.10.0", "1.11.0", "1.12.0", "1.12.1", "1.13.0", "1.14.0"];

    for release in releases.iter() {
        contributors::create_release(&connection, release);
    }

    // create most commits
    //
    // due to the way git works, this will not create any commits that were backported
    // so we'll do those below
    let git_log = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("--no-pager")
        .arg("log")
        .arg("--use-mailmap")
        .arg(r#"--format=%H %ae %an"#)
        .arg("master")
        .output()
        .expect("failed to execute process");

    let git_log = git_log.stdout;
    let git_log = String::from_utf8(git_log).unwrap();

    for log_line in git_log.split('\n') {
        // there is a last, blank line
        if log_line == "" {
            continue;
        }

        let mut split = log_line.splitn(3, ' ');

        let sha = split.next().unwrap();
        let author_email = split.next().unwrap();
        let author_name = split.next().unwrap();

        info!(log, "Creating commit: {}", sha);

        // We tag all commits initially to the first release. Each release will
        // set this properly below.
        contributors::create_commit(&connection, &sha, &author_name, &author_email, &first_release);
    }

    // assign commits to their release
    contributors::assign_commits(&log, "0.2", "0.1", &path);
    contributors::assign_commits(&log, "0.3", "0.2", &path);
    contributors::assign_commits(&log, "0.4", "0.3", &path);
    contributors::assign_commits(&log, "0.5", "0.4", &path);
    contributors::assign_commits(&log, "0.6", "0.5", &path);
    contributors::assign_commits(&log, "0.7", "0.6", &path);
    contributors::assign_commits(&log, "0.8", "0.7", &path);
    contributors::assign_commits(&log, "0.9", "0.8", &path);
    contributors::assign_commits(&log, "0.10", "0.9", &path);
    contributors::assign_commits(&log, "0.11.0", "0.10", &path);
    contributors::assign_commits(&log, "0.12.0", "0.11.0", &path);
    contributors::assign_commits(&log, "1.0.0-alpha", "0.12.0", &path);
    contributors::assign_commits(&log, "1.0.0-alpha.2", "1.0.0-alpha", &path);
    contributors::assign_commits(&log, "1.0.0-beta", "1.0.0-alpha.2", &path);
    contributors::assign_commits(&log, "1.0.0", "1.0.0-beta", &path);
    contributors::assign_commits(&log, "1.1.0", "1.0.0", &path);
    contributors::assign_commits(&log, "1.2.0", "1.1.0", &path);
    contributors::assign_commits(&log, "1.3.0", "1.2.0", &path);
    contributors::assign_commits(&log, "1.4.0", "1.3.0", &path);
    contributors::assign_commits(&log, "1.5.0", "1.4.0", &path);
    contributors::assign_commits(&log, "1.6.0", "1.5.0", &path);
    contributors::assign_commits(&log, "1.7.0", "1.6.0", &path);
    contributors::assign_commits(&log, "1.8.0", "1.7.0", &path);
    contributors::assign_commits(&log, "1.9.0", "1.8.0", &path);
    contributors::assign_commits(&log, "1.10.0", "1.9.0", &path);
    contributors::assign_commits(&log, "1.11.0", "1.10.0", &path);
    contributors::assign_commits(&log, "1.12.0", "1.11.0", &path);
    contributors::assign_commits(&log, "1.12.1", "1.12.0", &path);
    contributors::assign_commits(&log, "1.13.0", "1.12.0", &path);
    contributors::assign_commits(&log, "1.14.0", "1.13.0", &path);
    contributors::assign_commits(&log, "master", "1.14.0", &path);

    info!(log, "Done!");
}
