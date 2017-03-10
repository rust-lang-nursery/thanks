extern crate thanks;

extern crate clap;

extern crate diesel;

#[macro_use]
extern crate slog;
extern crate slog_term;

use clap::{App, Arg};
use slog::DrainExt;

use diesel::prelude::*;

fn main() {
    let matches = App::new("opt-out")
        .about("mark an author as opted-out")
        .arg(Arg::with_name("email")
             .short("e")
             .long("email")
             .takes_value(true)
             .required(true))
        .arg(Arg::with_name("opt-in")
             .long("opt-in")
             .help("Use this to mark author as opted-in again"))
        .get_matches();

    let log = slog::Logger::root(slog_term::streamer().full().build().fuse(), o!("version" => env!("CARGO_PKG_VERSION")));

    let visible = matches.is_present("opt-in");

    match matches.value_of("email") {
        Some(email) => opt_out(&log, email, visible),
        None => error!(log, "No email specified")
    };
}

fn opt_out(log: &slog::Logger, author_email: &str, new_visible: bool) {
    use thanks::schema::authors::dsl::*;
    use thanks::models::Author;
    let connection = thanks::establish_connection();

    diesel::update(authors.filter(email.eq(author_email)))
        .set(visible.eq(new_visible))
        .get_result::<Author>(&connection)
        .expect(&format!("Unable to find author with email {}", author_email));

    match new_visible {
        true => info!(log, "Opted-in author with email: {}", author_email),
        false => info!(log, "Opted-out author with email: {}", author_email),
    }
}
