extern crate contributors;

extern crate diesel;

use diesel::prelude::*;

use std::process::Command;
use std::env;

fn main() {
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


    let path = env::args().nth(1).unwrap();
    println!("Path to rust repo: {}", path);

    let git_log = Command::new("git")
        .current_dir(path)
        .arg("--no-pager")
        .arg("log")
        .arg(r#"--format=":%H""#)
        .arg(&format!("{}...{}", release.version, new_release_name))
        .output()
        .expect("failed to execute process");

    let shas = git_log.stdout;
    println!("OUT: {}", String::from_utf8(shas).unwrap());
    
    //let release = contributors::create_release(&connection, "1.14.0");
    //println!("\nCreated release {}", release.version);
}
