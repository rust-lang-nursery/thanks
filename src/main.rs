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

static RELEASES: [(&'static str, &'static str); 33] = [
    ("1.16.0", "1.15.0"),
    ("1.15.1", "1.15.0"),
    ("1.15.0", "1.14.0"),
    ("1.14.0", "1.13.0"),
    ("1.13.0", "1.12.0"),
    ("1.12.0", "1.11.0"),
    ("1.11.0", "1.10.0"),
    ("1.10.0", "1.9.0"),
    ("1.9.0", "1.8.0"),
    ("1.8.0", "1.7.0"),
    ("1.7.0", "1.6.0"),
    ("1.6.0", "1.5.0"),
    ("1.5.0", "1.4.0"),
    ("1.4.0", "1.3.0"),
    ("1.3.0", "1.2.0"),
    ("1.2.0", "1.1.0"),
    ("1.1.0", "1.0.0"),
    ("1.0.0", "1.0.0-beta"),
    ("1.0.0-beta", "1.0.0-alpha.2"),
    ("1.0.0-alpha.2", "1.0.0-alpha"),
    ("1.0.0-alpha", "0.12.0"),
    ("0.12.0", "0.11.0"),
    ("0.11.0", "0.10"),
    ("0.10", "0.9"),
    ("0.9", "0.8"),
    ("0.8", "0.7"),
    ("0.7", "0.6"),
    ("0.6", "0.5"),
    ("0.5", "0.4"),
    ("0.4", "0.3"),
    ("0.3", "0.2"),
    ("0.2", "0.1"),
    ("0.1", "c01efc669f09508b55eced32d3c88702578a7c3e"), // first commit
];

fn main() {
    // reference this to initialize it
    &REPOSITORY;

    println!("Starting up.");

    let mut server = sparkles::Server::new("templates".to_string());

    server.add_route("/", routes::root);
    server.add_route("/about", routes::about);
    server.add_route("/rust/all-time", routes::all_time);
    server.add_regex_route("/([^/]+)/(.+)", routes::release);

    let addr = "0.0.0.0:8080".parse().unwrap();
    server.run(&addr);
}
