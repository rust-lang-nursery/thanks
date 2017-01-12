extern crate contributors;

extern crate diesel;

fn main() {
    let connection = contributors::establish_connection();

    let release = contributors::create_release(&connection, "1.14.0");
    println!("\nCreated release {}", release.version);
}
