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

use std::io;
use std::process::{Command, Stdio};

fn main() {
    let matches = App::new("populate")
        .about("initialize the database")
        .arg(Arg::with_name("filepath")
            .short("p")
            .long("path")
            .help("filepath of the source code")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("url_path")
            .short("u")
            .long("url")
            .help("url path for this project")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("name")
            .short("n")
            .long("name")
            .help("name of the project")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("github_name")
            .short("g")
            .long("github")
            .help("GitHub link of the project")
            .takes_value(true)
            .required(true))
        .get_matches();

    let log = slog::Logger::root(slog_term::streamer().full().build().fuse(), o!("version" => env!("CARGO_PKG_VERSION")));

    let connection = contributors::establish_connection();

    // get name
    let project_name = matches.value_of("name").unwrap();
    info!(log, "Project name: {}", project_name);

    // check that we have no releases for given project
    {
        use contributors::models::Release;
        use contributors::schema::projects::dsl::*;
        use contributors::models::Project;

        if let Ok(project) = projects.filter(name.eq(project_name)).load::<Project>(&connection) {
            if let Ok(count) = Release::belonging_to(&project).count().first::<i64>(&connection) {
                if count > 0 {
                    panic!("you have releases in here already");
                }
            }
        }
    }

    // check that we have no commits
    {
        // if there are no releases then there should be no commits as well
        // so we may skip this check
        // I consider changing release_id to NOT NULL since we assign commit
        // to the first release on creation
    }

    // get path to git repo
    let path = matches.value_of("filepath").unwrap();
    info!(log, "Path to project's repo: {}", path);

    // get url path
    let url_path = matches.value_of("url_path").unwrap();
    info!(log, "URL path: {}", url_path);

    // get github name
    let github_name = matches.value_of("github_name").unwrap();
    info!(log, "GitHub name: {}", github_name);

    // create project
    let project = contributors::projects::create(&connection, project_name, url_path, github_name);

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

    let git_log = git_log.stdout;
    let git_log = String::from_utf8(git_log).unwrap();
    {
        use contributors::schema::releases::dsl::*;
        use contributors::models::Release;
        let first_release = releases.
            filter(project_id.eq(project.id)).
            first::<Release>(&connection).
            expect("No release found!");

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
    }

    // assign commits to their release
    let master = "master";
    for (i, v1) in releases.iter().enumerate() {
        let v2 = releases.get(i+1).unwrap_or(&master);
        println!("({}, {})", v1, v2);
        contributors::assign_commits(&log, v2, v1, project.id, &path);
    }


    info!(log, "Done!");
}
