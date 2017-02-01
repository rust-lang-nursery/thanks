extern crate thanks;

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

    let connection = thanks::establish_connection();

    match matches.is_present("all") {
        true => delete_whole_db(&log, &connection),
        false => {
            match matches.value_of("project_name") {
                Some(project_name) => delete_projects_db(&log, &connection, project_name),
                None => println!("No project specified"),
            };
        }
    }
}

fn delete_projects_db(log: &slog::Logger, connection: &PgConnection, project_name: &str) {
    use thanks::schema::releases::dsl::*;
    use thanks::models::Release;
    use thanks::schema::projects::dsl::{projects, name};
    use thanks::models::Project;
    use thanks::schema::commits::dsl::{commits, release_id};
    use diesel::expression::dsl::any;

    let project = projects.filter(name.eq(project_name)).first::<Project>(connection).expect("Unknown project!");
    let releases_to_delete = Release::belonging_to(&project).load::<Release>(connection).unwrap();
    let release_names: Vec<i32> = releases_to_delete.iter().map(|ref release| release.id).collect();
    let release_ids: Vec<i32> = releases_to_delete.iter().map(|ref release| release.id).collect();
    info!(log, "Deleting project {} with release names: {:?}", project_name, release_names);

    info!(log, "Deleting commits");
    diesel::delete(commits.filter(release_id.eq(any(&release_ids))))
        .execute(connection)
        .expect("Error deleting releases");

    info!(log, "Deleting releases");
    diesel::delete(releases.filter(id.eq(any(&release_ids))))
        .execute(connection)
        .expect("Error deleting releases");

    info!(log, "Deleting projects");
    diesel::delete(projects.filter(name.eq(project_name)))
        .execute(connection)
        .expect("Error deleting projects");

    info!(log, "Done.");
}

fn delete_whole_db(log: &slog::Logger, connection: &PgConnection) {
    use thanks::schema::releases::dsl::*;
    use thanks::schema::commits::dsl::*;
    use thanks::schema::projects::dsl::*;

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
