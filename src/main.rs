extern crate futures;
extern crate handlebars;
extern crate hyper;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use hyper::{Get, StatusCode};
use hyper::server::{Server, Service, Request, Response};

use handlebars::Handlebars;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::io::prelude::*;
use std::fs::File;

use serde_json::value::Value;

struct Contributors;

#[derive(Deserialize)]
struct GitHubResponse {
    url: String,
    total_commits: u32,
    commits: Vec<Commit>
}

#[derive(Deserialize)]
struct Commit {
    sha: String,
    commit: CommitData,
}

#[derive(Deserialize)]
struct CommitData {
    author: Author,
}

#[derive(Deserialize)]
struct Author {
    name: String,
    email: String,
    date: String,
}

impl Service for Contributors {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = ::futures::Finished<Response, hyper::Error>;

    fn call(&self, req: Request) -> Self::Future {
        ::futures::finished(match (req.method(), req.path()) {
            (&Get, Some("/")) => {
                let handlebars = Handlebars::new();

                let mut f = File::open("templates/index.hbs").unwrap();
                let mut source = String::new();

                f.read_to_string(&mut source).unwrap();

                let data: BTreeMap<String, String> = BTreeMap::new();
                // data.insert("world".to_string(), "world!".to_string());

                Response::new()
                    .with_body(handlebars.template_render(&source, &data).unwrap())
            },
            (&Get, Some(path)) => {
                let handlebars = Handlebars::new();

                let mut f = File::open("templates/release.hbs").unwrap();
                let mut source = String::new();

                f.read_to_string(&mut source).unwrap();

                let mut data: BTreeMap<String, Value> = BTreeMap::new();
                // strip the leading `/` lol
                data.insert("release".to_string(), Value::String(path[1..].to_string()));

                let mut resp = reqwest::get("https://api.github.com/repos/rust-lang/rust/compare/1.13.0...1.14.0").unwrap();

                let json: GitHubResponse = resp.json().unwrap();

                data.insert("url".to_string(), Value::String(json.url));

                let mut authors = HashSet::new();

                for commit in json.commits {
                    authors.insert(commit.commit.author.name);
                }

                let mut authors: Vec<_> = authors.into_iter().collect();

                authors.sort();

                let authors = authors.into_iter()
                    .map(|s| Value::String(s))
                    .collect();

                data.insert("authors".to_string(), Value::Array(authors));

                Response::new()
                    .with_body(handlebars.template_render(&source, &data).unwrap())
            },
            _ => {
                Response::new()
                    .with_status(StatusCode::NotFound)
            }
        })
    }

}


fn main() {
    let addr = "127.0.0.1:1337".parse().unwrap();
    let (listening, server) = Server::standalone(|tokio| {
        Server::http(&addr, tokio)?
            .handle(|| Ok(Contributors), tokio)
    }).unwrap();
    println!("Listening on http://{}", listening);
    server.run();
}

