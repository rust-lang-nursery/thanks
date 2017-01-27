extern crate contributors;
extern crate diesel;

extern crate dotenv;

extern crate futures;

extern crate handlebars;

extern crate hyper;
extern crate reqwest;

extern crate serde;
extern crate serde_json;

extern crate http;

use hyper::StatusCode;
use hyper::header::ContentType;
use hyper::server::{Request, Response};

use handlebars::Handlebars;

use std::collections::BTreeMap;
use std::env;
use std::io::prelude::*;
use std::fs::File;

use serde_json::value::Value;

fn root(_: Request) -> futures::Finished<Response, hyper::Error> {
    let handlebars = Handlebars::new();

    let mut f = File::open("templates/index.hbs").unwrap();
    let mut source = String::new();

    f.read_to_string(&mut source).unwrap();

    let mut data: BTreeMap<String, Value> = BTreeMap::new();

    data.insert("releases".to_string(), Value::Array(contributors::releases()));

    ::futures::finished(Response::new()
                        .with_header(ContentType::html())
                        .with_body(handlebars.template_render(&source, &data).unwrap())
                       )
}

fn about(_: Request) -> futures::Finished<Response, hyper::Error> {
    let handlebars = Handlebars::new();

    let mut f = File::open("templates/about.hbs").unwrap();
    let mut source = String::new();

    f.read_to_string(&mut source).unwrap();

    let data: BTreeMap<String, Value> = BTreeMap::new();

    ::futures::finished(Response::new()
                        .with_header(ContentType::html())
                        .with_body(handlebars.template_render(&source, &data).unwrap())
                       )
}

fn all_time(_: Request) -> futures::Finished<Response, hyper::Error> {
    let handlebars = Handlebars::new();

    let mut source = String::new();

    let mut data: BTreeMap<String, Value> = BTreeMap::new();

    let mut f = File::open("templates/all-time.hbs").unwrap();

    f.read_to_string(&mut source).unwrap();

    let scores = contributors::scores();

    data.insert("release".to_string(), Value::String(String::from("all-time")));
    data.insert("count".to_string(), Value::U64(scores.len() as u64));
    data.insert("scores".to_string(), Value::Array(scores));

    ::futures::finished(Response::new()
                        .with_header(ContentType::html())
                        .with_body(handlebars.template_render(&source, &data).unwrap())
                       )
}

fn catch_all(req: Request) -> futures::Finished<Response, hyper::Error> {
    let path = req.path();

    let handlebars = Handlebars::new();

    let mut source = String::new();

    let mut data: BTreeMap<String, Value> = BTreeMap::new();

    // strip the leading `/` lol
    let release_name = path[1..].to_string();

    data.insert("release".to_string(), Value::String(release_name.clone()));

    let mut f = File::open("templates/release.hbs").unwrap();
    f.read_to_string(&mut source).unwrap();

    let names = contributors::names(&release_name);

    match names {
        Some(names) => {
            data.insert("count".to_string(), Value::U64(names.len() as u64));
            data.insert("names".to_string(), Value::Array(names));
        },
        None => {
            return ::futures::finished(Response::new()
                                       .with_status(StatusCode::NotFound));
        }
    }

    ::futures::finished(Response::new()
                        .with_header(ContentType::html())
                        .with_body(handlebars.template_render(&source, &data).unwrap())
                       )
}

fn main() {
    dotenv::dotenv().ok();

    let addr = format!("0.0.0.0:{}", env::args().nth(1).unwrap_or(String::from("1337"))).parse().unwrap();

    let server = http::Server;
    let mut contributors = http::Contributors::new();

    contributors.add_route("/", root);

    contributors.add_route("/about", about);

    contributors.add_route("/all-time", all_time);
    
    // * is the catch-all route
    contributors.add_route("*", catch_all);

    println!("Starting server, listening on http://{}", addr);

    server.run(&addr, contributors);
}
