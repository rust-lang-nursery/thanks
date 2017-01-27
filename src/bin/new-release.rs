extern crate contributors;

extern crate diesel;
extern crate clap;

use diesel::prelude::*;
use clap::{App, Arg};

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

    // get name
    let project_name = matches.value_of("project_name").unwrap();
    println!("Project name: {}", project_name);
    // get version
    let new_release_name = matches.value_of("version").unwrap();
    println!("New version: {}", project_name);
    // get path
    let path = matches.value_of("filepath").unwrap();
    println!("Path to {} repo: {}", project_name, path);

    use contributors::schema::releases::dsl::*;
    use contributors::models::Release;
    use contributors::schema::projects::dsl::{projects, name};
    use contributors::models::Project;

    let connection = contributors::establish_connection();

    let project = projects.filter(name.eq(project_name)).first::<Project>(&connection).expect("Unknown project!");
    let release = Release::belonging_to(&project).order(id.desc()).first::<Release>(&connection).unwrap();

    println!("Previous release: {}", release.version);
    println!("Creating new release release: {}", new_release_name);

    if Release::belonging_to(&project).filter(version.eq(&new_release_name)).first::<Release>(&connection).is_ok() {
       panic!("Release {} already exists! Something must be wrong.", new_release_name);
    }

    let new_release = contributors::create_release(&connection, &new_release_name, project.id);
    println!("Created release {}", new_release.version);

    println!("Assigning commits for {}", new_release.version);
    contributors::assign_commits(&new_release.version, &release.version, project.id, &path);
}
