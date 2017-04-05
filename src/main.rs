extern crate futures;
extern crate sparkles;
extern crate thanks;
extern crate git2;

mod routes;

use std::fs;
use std::path::PathBuf;

fn main() {
    let checkout = PathBuf::from("tmp/rust");
    let url = "https://github.com/rust-lang/rust";

    println!("Initializing");

    let repo = match git2::Repository::open(&checkout) {
        Ok(r) => r,
        Err(..) => {
            println!("Fetching repo");
            let _ = fs::remove_dir_all(&checkout);
            fs::create_dir_all(&checkout).unwrap();
            git2::build::RepoBuilder::new()
                .bare(true)
                .clone(&url, &checkout).unwrap()
        }
    };

    println!("Starting up.");

    let mut server = sparkles::Server::new("templates".to_string());

    server.add_route("/", routes::root);

    let addr = "0.0.0.0:8080".parse().unwrap();
    server.run(&addr);
}
