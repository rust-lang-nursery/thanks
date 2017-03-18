extern crate thanks;

extern crate dotenv;

extern crate futures;

extern crate handlebars;

extern crate hyper;

extern crate regex;

extern crate serde_json;

extern crate http;

use http::Request;
use http::Response;
use http::ResponseBuilder;
use http::Status;

use regex::Captures;

use std::env;

use serde_json::value::Value;

fn main() {
    dotenv::dotenv().ok();

    let addr = format!("0.0.0.0:{}",
                       env::args().nth(1).unwrap_or(String::from("1337")))
        .parse()
        .unwrap();

    let mut server = http::Server::new("templates".to_string());

    server.add_route("/", root);

    server.add_route("/about", about);

    server.add_route("/rust/all-time", all_time);

    server.add_regex_route("/([^/]+)/(.+)", release);

    server.run(&addr);
}

fn root(_: Request) -> Response {
    let mut res = ResponseBuilder::new();
    res.with_template("index.hbs".to_string());

    res.data.insert("maintenance".to_string(),
                Value::Bool(thanks::in_maintenance()));

    res.data.insert("releases".to_string(),
                Value::Array(thanks::releases::all()));

    res.with_status(Status::Ok);

    res.to_response()
}

fn about(_: Request) -> Response {
    let mut res = ResponseBuilder::new();
    res.with_template("about.hbs".to_string());

    res.data.insert("maintenance".to_string(),
                Value::Bool(thanks::in_maintenance()));

    res.with_status(Status::Ok);

    res.to_response()
}

fn all_time(_: Request) -> Response {
    let mut res = ResponseBuilder::new();
    res.with_template("all-time.hbs".to_string());

    res.data.insert("maintenance".to_string(),
                Value::Bool(thanks::in_maintenance()));

    let scores = thanks::scores();

    res.data.insert("release".to_string(),
                Value::String(String::from("all-time")));
    res.data.insert("count".to_string(), Value::Number((scores.len() as u64).into()));
    res.data.insert("scores".to_string(), Value::Array(scores));

    res.with_status(Status::Ok);

    res.to_response()
}

fn release(_: &Request, cap: Captures) -> Response {
    let mut res = ResponseBuilder::new();
    res.with_template("release.hbs".to_string());

    res.data.insert("maintenance".to_string(),
                Value::Bool(thanks::in_maintenance()));

    let project = cap.get(1).unwrap();
    let project = project.as_str();

    let release_name = cap.get(2).unwrap();
    let release_name = release_name.as_str();

    res.data.insert("release".to_string(), Value::String(release_name.to_string()));

    let names = thanks::releases::contributors(project, release_name);

    match names {
        Some(names) => {
            res.data.insert("count".to_string(), Value::Number((names.len() as u64).into()));
            res.data.insert("names".to_string(), Value::Array(names));
            res.with_status(Status::Ok);
        }
        None => {
            res.with_status(Status::NotFound);
        }
    }

    res.to_response()
}