extern crate thanks;

extern crate dotenv;

extern crate futures;

extern crate handlebars;

extern crate hyper;

extern crate regex;

extern crate serde_json;

#[macro_use]
extern crate slog;
extern crate slog_term;

extern crate http;

use http::Request;
use http::Response;

use regex::Captures;

use std::env;

use serde_json::value::Value;

use slog::DrainExt;

// Rename type for crate
type BTreeMap = std::collections::BTreeMap<String, Value>;

fn main() {
    dotenv::dotenv().ok();

    let log = slog::Logger::root(slog_term::streamer().full().build().fuse(),
                                 o!("version" => env!("CARGO_PKG_VERSION")));

    let addr = format!("0.0.0.0:{}",
                       env::args().nth(1).unwrap_or(String::from("1337")))
        .parse()
        .unwrap();

    let server = http::Server;

    let mut thanks = http::Contributors::new("templates".to_string());

    thanks.add_route("/", root);

    thanks.add_route("/about", about);

    thanks.add_route("/rust/all-time", all_time);

    thanks.add_regex_route("/([^/]+)/(.+)", release);

    info!(log, "Starting server, listening on http://{}", addr);

    server.run(&addr, thanks);
}

fn root(_: Request) -> Response {
    let mut data: BTreeMap = BTreeMap::new();

    data.insert("maintenance".to_string(),
                Value::Bool(thanks::in_maintenance()));

    data.insert("releases".to_string(),
                Value::Array(thanks::releases::all()));

    Response::Success {
        data: data,
        template: "index.hbs".to_string(),
    }
}

fn about(_: Request) -> Response {
    let mut data: BTreeMap = BTreeMap::new();

    data.insert("maintenance".to_string(),
                Value::Bool(thanks::in_maintenance()));

    Response::Success {
        data: data,
        template: "about.hbs".to_string(),
    }
}

fn all_time(_: Request) -> Response {
    let mut data: BTreeMap = BTreeMap::new();

    data.insert("maintenance".to_string(),
                Value::Bool(thanks::in_maintenance()));

    let scores = thanks::scores();

    data.insert("release".to_string(),
                Value::String(String::from("all-time")));
    data.insert("count".to_string(), Value::Number((scores.len() as u64).into()));
    data.insert("scores".to_string(), Value::Array(scores));

    Response::Success {
        data: data,
        template: "all-time.hbs".to_string(),
    }
}

fn release(_: &Request, cap: Captures) -> Response {
    let mut data: BTreeMap = BTreeMap::new();

    data.insert("maintenance".to_string(),
                Value::Bool(thanks::in_maintenance()));

    let project = cap.get(1).unwrap();
    let project = project.as_str();

    let release_name = cap.get(2).unwrap();
    let release_name = release_name.as_str();

    data.insert("release".to_string(), Value::String(release_name.to_string()));

    let names = thanks::releases::contributors(project, release_name);

    match names {
        Some(names) => {
            data.insert("count".to_string(), Value::Number((names.len() as u64).into()));
            data.insert("names".to_string(), Value::Array(names));
        }
        None => {
            return Response::NotFound;
        }
    }

    Response::Success {
        data: data,
        template: "release.hbs".to_string(),
    }
}