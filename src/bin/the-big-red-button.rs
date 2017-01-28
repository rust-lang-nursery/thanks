extern crate contributors;

extern crate clap;

extern crate diesel;

#[macro_use]
extern crate slog;
extern crate slog_term;

use clap::{App, Arg, ArgGroup};
use slog::DrainExt;

use diesel::prelude::*;
use diesel::pg::PgConnection;

fn main() {
    let matches = App::new("the-big-red-button")
        .about("annihilate")
        .arg(Arg::with_name("all")
            .long("all")
            .help("remove everything from the database")
            .conflicts_with("project_name"))
        .arg(Arg::with_name("project_name")
            .short("n")
            .long("name")
            .help("name of the project to delete")
            .conflicts_with("all")
            .takes_value(true))
        .group(ArgGroup::with_name("opts")
               .args(&["all", "project_name"])
               .required(true))
        .get_matches();

    let log = slog::Logger::root(slog_term::streamer().full().build().fuse(), o!("version" => env!("CARGO_PKG_VERSION")));

    let connection = contributors::establish_connection();

    match matches.is_present("all") {
        true => delete_whole_db(&log, &connection),
        false => {
            match matches.value_of("project_name") {
                Some(project_name) => info!(&log, "Project name: {}", project_name),
                None => println!("No project specified"),
            };
        }
    }
}

fn delete_whole_db(log: &slog::Logger, connection: &PgConnection) {
    use contributors::schema::releases::dsl::*;
    use contributors::schema::commits::dsl::*;
    use contributors::schema::projects::dsl::*;

    info!(log, "Deleting commits");
    diesel::delete(commits)
        .execute(connection)
        .expect("Error deleting releases");

    info!(log, "Deleting releases");
    diesel::delete(releases)
        .execute(connection)
        .expect("Error deleting releases");

    info!(log, "Deleting projects");
    diesel::delete(projects)
        .execute(connection)
        .expect("Error deleting projects");

    info!(log, "Done.");

}
