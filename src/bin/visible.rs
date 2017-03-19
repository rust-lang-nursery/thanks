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
    let matches = App::new("visible")
        .about("mark a release as visible")
        .arg(Arg::with_name("version")
            .short("v")
            .long("version")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("hide")
            .long("hide")
            .help("Use this to mark the release as hidden"))
        .get_matches();

    let log = slog::Logger::root(slog_term::streamer().full().build().fuse(), o!("version" => env!("CARGO_PKG_VERSION")));

    let visible = ! matches.is_present("hide");
    
    match matches.value_of("version") {
        Some(version) => set_visibility(&log, version, visible),
        None => error!(log, "No version specified"),
    }
}

fn set_visibility(log: &slog::Logger, release_version: &str, new_visible: bool) {
    use thanks::schema::releases::dsl::*;
    use thanks::models::Release;
    let connection = thanks::establish_connection();

    diesel::update(releases.filter(version.eq(release_version)))
        .set(visible.eq(new_visible))
        .get_result::<Release>(&connection)
        .expect(&format!("Unable to find release with version {}", release_version));

    match new_visible {
        true => info!(log, "Set version {} to show.", release_version),
        false => info!(log, "Set version {} to hide.", release_version),
    }
}

