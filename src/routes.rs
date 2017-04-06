use caseless;
use futures::BoxFuture;
use sparkles::{Request, Response, ResponseBuilder, Status, Error};
use REPOSITORY;
use serde_json::{Value, Map};
use std::collections::{HashSet, HashMap};
use std::cmp::Ordering;
use regex::Captures;
use MAILMAP;
use RELEASES;
use thanks;

/// The handler for /
pub fn root(_: Request) -> BoxFuture<Response, Error> {
    let mut res = ResponseBuilder::new();
    res.with_template("index".to_string());

    let mut releases = vec!["master"];
    releases.extend(RELEASES.into_iter().map(|&(a, _, _)| a));

    res.data.insert("releases".to_string(),
                Value::from(releases));

    res.with_status(Status::Ok);

    res.to_response().into_future()
}

/// for /{{project}}/{{release}}
pub fn release(_: &Request, cap: Captures) -> BoxFuture<Response, Error> {
    let mut res = ResponseBuilder::new();
    res.with_template("release".to_string());

    let project = cap.get(1).unwrap();
    let project = project.as_str();

    let release_name = cap.get(2).unwrap();
    let release_name = release_name.as_str();

    res.data.insert("release".to_string(), Value::String(release_name.to_string()));

    let (release_name, previous, notes) = if release_name == "master" {
        ("HEAD", RELEASES[0].1, "https://github.com/rust-lang/rust/blob/master/RELEASES.md")
    } else {
        *RELEASES.iter().find(|&&(r, p, n)| r == release_name).unwrap()
    };

    let names = thanks::names(release_name, previous, &REPOSITORY.lock().unwrap(), &MAILMAP);

    res.data.insert("count".to_string(), Value::Number((names.len() as u64).into()));
    res.data.insert("names".to_string(), Value::Array(names));
    res.data.insert("link".to_string(), Value::String(notes.to_string()));
    res.with_status(Status::Ok);

    res.to_response().into_future()
}

pub fn about(_: Request) -> BoxFuture<Response, Error> {
    let mut res = ResponseBuilder::new();
    res.with_template("about".to_string());

    res.with_status(Status::Ok);

    res.to_response().into_future()
}

pub fn all_time(_: Request) -> BoxFuture<Response, Error> {
    let mut res = ResponseBuilder::new();
    res.with_template("all-time".to_string());

    let scores = thanks::scores(&REPOSITORY.lock().unwrap(), &MAILMAP);

    res.data.insert("release".to_string(),
                Value::String(String::from("all-time")));
    res.data.insert("count".to_string(), Value::Number((scores.len() as u64).into()));
    res.data.insert("scores".to_string(), Value::Array(scores));

    res.with_status(Status::Ok);

    res.to_response().into_future()
}