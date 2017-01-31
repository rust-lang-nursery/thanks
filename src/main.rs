extern crate contributors;

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

use hyper::StatusCode;
use hyper::header::ContentType;
use hyper::server::{Request, Response};

use handlebars::Handlebars;

use regex::Captures;

use std::env;
use std::path::Path;

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
    let mut contributors = http::Contributors::new();

    contributors.add_route("/", root);

    contributors.add_route("/about", about);

    contributors.add_route("/rust/all-time", all_time);
    
    contributors.add_regex_route("/([^/]+)/(.+)", release);

    info!(log, "Starting server, listening on http://{}", addr);

    server.run(&addr, contributors);
}

fn root(_: Request) -> futures::Finished<Response, hyper::Error> {
    let mut data: BTreeMap = BTreeMap::new();


    data.insert("releases".to_string(),
                Value::Array(contributors::releases()));

    let template = build_template(&data, "templates/index.hbs");

    ::futures::finished(Response::new()
        .with_header(ContentType::html())
        .with_body(template))
}

fn about(_: Request) -> futures::Finished<Response, hyper::Error> {
    let data: BTreeMap = BTreeMap::new();

    let template = build_template(&data, "templates/about.hbs");

    ::futures::finished(Response::new()
        .with_header(ContentType::html())
        .with_body(template))
}

fn all_time(_: Request) -> futures::Finished<Response, hyper::Error> {
    let mut data: BTreeMap = BTreeMap::new();

    let scores = contributors::scores();

    data.insert("release".to_string(),
                Value::String(String::from("all-time")));
    data.insert("count".to_string(), Value::U64(scores.len() as u64));
    data.insert("scores".to_string(), Value::Array(scores));

    let template = build_template(&data, "templates/all-time.hbs");

    ::futures::finished(Response::new()
        .with_header(ContentType::html())
        .with_body(template))
}

fn release(_: &Request, cap: Captures) -> futures::Finished<Response, hyper::Error> {
    let mut data: BTreeMap = BTreeMap::new();

    let project = cap.get(1).unwrap();
    let project = project.as_str();

    let release_name = cap.get(2).unwrap();
    let release_name = release_name.as_str();

    data.insert("release".to_string(), Value::String(release_name.to_string()));

    let names = contributors::releases::contributors(project, release_name);

    match names {
        Some(names) => {
            data.insert("count".to_string(), Value::U64(names.len() as u64));
            data.insert("names".to_string(), Value::Array(names));
        }
        None => {
            return ::futures::finished(Response::new().with_status(StatusCode::NotFound));
        }
    }

    let template = build_template(&data, "templates/release.hbs");

    ::futures::finished(Response::new()
        .with_header(ContentType::html())
        .with_body(template))
}

/// Constructs Handlebars template from the provided variable data. Uses partial templates
/// to produce consistent container.
fn build_template(data: &BTreeMap, template_path: &str) -> String {
    let mut handlebars = Handlebars::new();
    // Render the partials
    handlebars.register_template_file("container", &Path::new("templates/container.hbs"))
        .ok()
        .unwrap();
    handlebars.register_template_file("index", &Path::new(template_path)).ok().unwrap();
    let mut data = data.clone();
    // Add name of the container to be loaded (just a constant for now)
    data.insert("parent".to_string(), Value::String("container".to_string()));

    // That's all we need to build this thing
    handlebars.render("index", &data).unwrap()

}
