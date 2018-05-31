extern crate thanks;

extern crate clap;

extern crate diesel;

#[macro_use]
extern crate slog;
extern crate slog_term;

use clap::{App, Arg};
use slog::DrainExt;

use diesel::prelude::*;

use thanks::models::Maintenance;

fn main() {
    let matches = App::new("maintenance")
        .about("let people know the db is re-building")
        .arg(
            Arg::with_name("on")
                .long("on")
                .help("turn maintenance on")
                .conflicts_with("off"),
        )
        .arg(
            Arg::with_name("off")
                .long("off")
                .help("turn maintenance off")
                .conflicts_with("on"),
        )
        .get_matches();

    let log = slog::Logger::root(
        slog_term::streamer().full().build().fuse(),
        o!("version" => env!("CARGO_PKG_VERSION")),
    );

    let connection = thanks::establish_connection();

    use thanks::schema::maintenances::dsl::*;
    let model = maintenances
        .find(1)
        .load::<Maintenance>(&connection)
        .expect("Error loading maintenance model")
        .remove(0);

    if matches.is_present("on") {
        diesel::update(&model)
            .set(enabled.eq(true))
            .get_result::<Maintenance>(&connection)
            .expect("Unable to update");
        info!(log, "maintenance turned on")
    } else if matches.is_present("off") {
        diesel::update(&model)
            .set(enabled.eq(false))
            .get_result::<Maintenance>(&connection)
            .expect("Unable to update");
        info!(log, "maintenance turned off")
    } else {
        panic!("you gotta say --on or --off");
    }
}
