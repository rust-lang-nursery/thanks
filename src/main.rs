extern crate caseless;
extern crate futures;
extern crate git2;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate serde_json;
extern crate sparkles;
extern crate thanks;
extern crate unicode_normalization;

mod routes;
mod mailmap;

use mailmap::Mailmap;
use std::io::BufReader;
use std::io::prelude::*;
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::Mutex;

lazy_static! {
    static ref REPOSITORY:  Mutex<git2::Repository> = {
        let checkout = PathBuf::from("tmp/rust");
        let url = "https://github.com/rust-lang/rust";

        println!("Initializing");

        Mutex::new(match git2::Repository::open(&checkout) {
            Ok(r) => r,
            Err(..) => {
                println!("Fetching repo");
                let _ = fs::remove_dir_all(&checkout);
                fs::create_dir_all(&checkout).unwrap();
                git2::build::RepoBuilder::new()
                    .bare(true)
                    .clone(&url, &checkout).unwrap()
            }
        })
    };
}

lazy_static! {
    static ref MAILMAP:  Mailmap = {
        let file_path = PathBuf::from("tmp/rust").join(".mailmap");
        let contents = {
            if file_path.is_file() {
                let file = File::open(file_path).unwrap();

                let mut buf_reader = BufReader::new(file);
                let mut contents = String::new();
                buf_reader.read_to_string(&mut contents).unwrap();
                contents
            } else {
                "".to_string()
            }
        };

        Mailmap::new(&contents)
    };
}

static RELEASES: [(&'static str, &'static str); 3] = [
    ("1.16.0", "1.15.0"),
    ("1.15.1", "1.15.0"),
    ("1.15.0", "1.14.0"),
];

fn main() {
    // reference this to initialize it
    &REPOSITORY;

    println!("Starting up.");

    let mut server = sparkles::Server::new("templates".to_string());

    server.add_route("/", routes::root);
    server.add_regex_route("/([^/]+)/(.+)", routes::release);

    let addr = "0.0.0.0:8080".parse().unwrap();
    server.run(&addr);
}
