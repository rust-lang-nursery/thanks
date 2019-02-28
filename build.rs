extern crate diesel;
extern crate diesel_migrations;
extern crate dotenv;

use diesel::prelude::*;
use diesel::pg::PgConnection;

use dotenv::dotenv;

use std::env;

fn main() {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let connection = PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url));

    diesel_migrations::run_pending_migrations(&connection)
        .expect("oh no migrations couldn't be run");
}
