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
        .get_matches();

    use contributors::schema::releases::dsl::*;
    use contributors::models::Release;

    let connection = contributors::establish_connection();

    let release: Release = releases.order(id.desc()).first(&connection).unwrap();

    let num: u64 = release.version.split(".").nth(1).unwrap().parse().unwrap();
    let new_release = num + 1;
    let new_release_name = format!("1.{}.0", new_release);

    println!("Previous release: {}", release.version);
    println!("Creating new release release: {}", new_release_name);

    if releases.filter(version.eq(&new_release_name)).first::<Release>(&connection).is_ok() {
       panic!("Release {} already exists! Something must be wrong.", new_release_name);
    }

    let path = matches.value_of("filepath").unwrap();
    println!("Path to rust repo: {}", path);

    let new_release = contributors::create_release(&connection, &new_release_name);
    println!("Created release {}", new_release.version);

    println!("Assigning commits for {}", new_release.version);
    contributors::assign_commits(&new_release.version, &release.version, &path);
}
