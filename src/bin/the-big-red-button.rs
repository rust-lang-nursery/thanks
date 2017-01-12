extern crate contributors;

extern crate diesel;

use diesel::prelude::*;

fn main() {
    use contributors::schema::releases::dsl::*;
    use contributors::schema::commits::dsl::*;

    let connection = contributors::establish_connection();

    println!("Deleting releases");
    diesel::delete(releases)
        .execute(&connection)
        .expect("Error deleting releases");

    println!("Deleting commits");
    diesel::delete(commits)
        .execute(&connection)
        .expect("Error deleting releases");

    println!("Done.");
}
