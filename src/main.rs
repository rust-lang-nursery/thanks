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

static RELEASES: [(&'static str, &'static str, &'static str); 34] = [
    ("1.16.0", "1.15.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1160-2017-03-16"),
    ("1.15.1", "1.15.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1151-2017-02-09"),
    ("1.15.0", "1.14.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1150-2017-02-02"),
    ("1.14.0", "1.13.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1140-2016-12-22"),
    ("1.13.0", "1.12.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1130-2016-11-10"),
    ("1.12.1", "1.11.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1121-2016-10-20"),
    ("1.12.0", "1.11.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1120-2016-09-29"),
    ("1.11.0", "1.10.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1110-2016-08-18"),
    ("1.10.0", "1.9.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1100-2016-07-07"),
    ("1.9.0", "1.8.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-190-2016-05-26"),
    ("1.8.0", "1.7.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-180-2016-04-14"),
    ("1.7.0", "1.6.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-170-2016-03-03"),
    ("1.6.0", "1.5.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-160-2016-01-21"),
    ("1.5.0", "1.4.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-150-2015-12-10"),
    ("1.4.0", "1.3.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-140-2015-10-29"),
    ("1.3.0", "1.2.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-130-2015-09-17"),
    ("1.2.0", "1.1.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-120-2015-08-07"),
    ("1.1.0", "1.0.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-110-2015-06-25"),
    ("1.0.0", "1.0.0-beta", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-100-2015-05-15"),
    ("1.0.0-beta", "1.0.0-alpha.2", "https://github.com/rust-lang/rust/blob/master/RELEASES.md"),
    ("1.0.0-alpha.2", "1.0.0-alpha", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-100-alpha2-2015-02-20"),
    ("1.0.0-alpha", "0.12.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-100-alpha-2015-01-09"),
    ("0.12.0", "0.11.0", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-0120-2014-10-09"),
    ("0.11.0", "0.10", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-0110-2014-07-02"),
    ("0.10", "0.9", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-010-2014-04-03"),
    ("0.9", "0.8", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-09-2014-01-09"),
    ("0.8", "0.7", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-08-2013-09-26"),
    ("0.7", "0.6", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-07-2013-07-03"),
    ("0.6", "0.5", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-06-2013-04-03"),
    ("0.5", "0.4", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-05-2012-12-21"),
    ("0.4", "0.3", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-04-2012-10-15"),
    ("0.3", "0.2", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-03--2012-07-12"),
    ("0.2", "0.1", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-02--2012-03-29"),
    // first commit
    ("0.1", "c01efc669f09508b55eced32d3c88702578a7c3e", "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-01--2012-01-20"),
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
