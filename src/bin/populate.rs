extern crate contributors;

extern crate diesel;

extern crate dotenv;

extern crate futures;

extern crate handlebars;

extern crate reqwest;

extern crate serde;
extern crate serde_json;

extern crate clap;

use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use clap::{App, Arg};

use std::env;
use std::io;
use std::process::{Command, Stdio};

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

fn main() {
    let matches = App::new("populate")
        .about("initialize the database")
        .arg(Arg::with_name("filepath")
            .short("p")
            .long("path")
            .help("filepath of the source code")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("name")
            .short("n")
            .long("name")
            .help("name of the project")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("github_link")
            .short("l")
            .long("link")
            .help("GitHub link of the project")
            .takes_value(true)
            .required(true))
        .get_matches();

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

    // get name
    let name = matches.value_of("name").unwrap();
    println!("Project name: {}", name);

    // get path to git repo
    let path = matches.value_of("filepath").unwrap();
    println!("Path to project's repo: {}", path);

    // get github link
    let link = matches.value_of("github_link").unwrap();
    println!("GitHub link: {}", link);

    // create project
    let project = contributors::create_project(&connection, name, path, link);

    // Create releases
    // Infer them from git tags
    let git_tags = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("tag")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to run command");
    let grep = Command::new("grep")
        .arg("-P")
        .arg("^\\d+.+|^v\\d+.+") // should this be configurable per project?
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to run command");
    let mut sort = Command::new("sort")
        .arg("-V")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failer to run command");
    io::copy(&mut git_tags.stdout.unwrap(), &mut grep.stdin.unwrap()).ok().expect("Cannot pipe");
    io::copy(&mut grep.stdout.unwrap(), sort.stdin.as_mut().unwrap()).ok().expect("Cannot pipe");
    let tags = sort.wait_with_output().unwrap().stdout;
    let releases = String::from_utf8(tags).unwrap();
    let releases = releases.lines();

    let releases: Vec<&str> = releases.collect();
    for release in releases.iter() {
        contributors::create_release(&connection, release, project.id);
    }
    // And create the release for all commits that are not released yet
    contributors::create_release(&connection, "master", project.id);

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

    let log = git_log.stdout;
    let log = String::from_utf8(log).unwrap();
    {
        use contributors::schema::releases::dsl::*;
        use contributors::models::Release;
        let first_release = releases.first::<Release>(&connection).expect("No release found!");

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
    }

    // assign commits to their release
    let master = "master";
    for (i, el1) in releases.iter().enumerate() {
        let el2 = releases.get(i+1).unwrap_or(&master);
        println!("({}, {})", el1, el2);
        contributors::assign_commits(el2, el1, &path);
    }

    println!("Done!");
}
