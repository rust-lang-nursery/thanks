extern crate thanks;

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
extern crate slog_async;

extern crate clap;
extern crate git2;

use diesel::pg::PgConnection;
use clap::{App, Arg, ArgMatches, SubCommand};
use slog::{Drain, Logger};

fn main() {
    let matches = App::new("projects manager")
        .about("add and remove projects")
        .subcommand(SubCommand::with_name("add")
            .about("adds a project to the database")
            .arg(Arg::with_name("name")
                 .short("n")
                 .long("name")
                 .help("name of the project")
                 .takes_value(true)
                 .required(true))
            .arg(Arg::with_name("url_path")
                 .short("u")
                 .long("url")
                 .help("url path for this project repository")
                 .takes_value(true)
                 .required(true))
            .arg(Arg::with_name("github_name")
                 .short("g")
                 .long("github")
                 .help("GitHub name of the project")
                 .takes_value(true)
                 .required(true))
            .arg(Arg::with_name("dir_path")
                 .short("d")
                 .long("dir")
                 .help("Directory path of the repository")
                 .takes_value(true)
                 .required(true))
            )
            .subcommand(SubCommand::with_name("remove")
                .about("remove a project from the database")
                .arg(Arg::with_name("name")
                     .short("n")
                     .long("name")
                     .help("name of the project")
                     .takes_value(true)
                     .required(true)
                    )
            )
            .get_matches();

    // Setup logging.
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let log = slog::Logger::root(drain, o!("version" => env!("CARGO_PKG_VERSION")));

    // Setup db connection.
    let connection = thanks::establish_connection();

    // Handle clap subcommands.
    match matches.subcommand() {
        ("add", Some(sub_m))    => add_project(&log, &connection, sub_m), 
        ("remove", Some(sub_m)) => remove_project(&log, &connection, sub_m), 
        _                       => println!("unrecognized command"),
    };

    info!(log, "Done!");
}

fn add_project(log: &Logger, connection: &PgConnection, matches: &ArgMatches) {
    // Get name.
    let project_name = matches.value_of("name").unwrap();
    info!(log, "Project name: {}", project_name);

    // Get url path.
    let url_path = matches.value_of("url_path").unwrap();
    info!(log, "URL path: {}", url_path);

    // Get github name.
    let github_name = matches.value_of("github_name").unwrap();
    info!(log, "GitHub name: {}", github_name);

    // Get repo directory path.
    let dir_path = matches.value_of("dir_path").unwrap();
    info!(log, "Directory path: {}", dir_path);

    // Create project.
    thanks::projects::create(connection, project_name, url_path, github_name, dir_path);
}

fn remove_project(log: &Logger, connection: &PgConnection, matches: &ArgMatches) {
    // Get name.
    let project_name = matches.value_of("name").unwrap();
    info!(log, "Project name: {}", project_name);

    // Remove project.
    thanks::projects::delete(connection, project_name);
}
