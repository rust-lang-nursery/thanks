extern crate thanks;

extern crate diesel;
extern crate clap;

#[macro_use]
extern crate slog;
extern crate slog_term;

use diesel::prelude::*;
use clap::{App, Arg};
use slog::DrainExt;

fn main() {
    let matches = App::new("new-release")
        .about("create a new release")
        .arg(Arg::with_name("filepath")
            .short("p")
            .long("path")
            .help("filepath of the rust source code")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("project_name")
            .short("n")
            .long("name")
            .help("name of the project")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("version")
            .short("v")
            .long("version")
            .help("new version number")
            .takes_value(true)
            .required(true))
        .get_matches();

    let log = slog::Logger::root(slog_term::streamer().full().build().fuse(), o!("version" => env!("CARGO_PKG_VERSION")));

    // get name
    let project_name = matches.value_of("project_name").unwrap();
    info!(&log, "Project name: {}", project_name);
    // get version
    let new_release_name = matches.value_of("version").unwrap();
    info!(&log, "New version: {}", project_name);
    // get path
    let path = matches.value_of("filepath").unwrap();
    info!(&log, "Path to {} repo: {}", project_name, path);

    use thanks::schema::releases::dsl::*;
    use thanks::models::Release;
    use thanks::schema::projects::dsl::{projects, name};
    use thanks::models::Project;

    let connection = thanks::establish_connection();

    let project = projects.filter(name.eq(project_name)).first::<Project>(&connection).expect("Unknown project!");
    let release = Release::belonging_to(&project).order(id.desc()).first::<Release>(&connection).unwrap();

    info!(log, "Previous release: {}", release.version);
    info!(log, "Creating new release: {}", new_release_name);

    if Release::belonging_to(&project).filter(version.eq(&new_release_name)).first::<Release>(&connection).is_ok() {
       panic!("Release {} already exists! Something must be wrong.", new_release_name);
    }

    let new_release = thanks::releases::create(&connection, &new_release_name, project.id, true);
    info!(log, "Created release {}", new_release.version);

    info!(log, "Assigning commits for {}", new_release.version);
    thanks::releases::assign_commits(&log, &new_release.version, &release.version, project.id, &path);
}
