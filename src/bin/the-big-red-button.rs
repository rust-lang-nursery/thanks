extern crate contributors;

extern crate diesel;

#[macro_use]
extern crate slog;
extern crate slog_term;

use slog::DrainExt;

use diesel::prelude::*;

fn main() {
    let log = slog::Logger::root(slog_term::streamer().full().build().fuse(), o!("version" => env!("CARGO_PKG_VERSION")));
    use contributors::schema::releases::dsl::*;
    use contributors::schema::commits::dsl::*;

    let connection = contributors::establish_connection();

    info!(log, "Deleting releases");
    diesel::delete(releases)
        .execute(&connection)
        .expect("Error deleting releases");

    info!(log, "Deleting commits");
    diesel::delete(commits)
        .execute(&connection)
        .expect("Error deleting releases");

    info!(log, "Done.");
}
