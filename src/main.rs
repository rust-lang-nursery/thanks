#[macro_use]
extern crate diesel;

extern crate futures;

extern crate handlebars;

extern crate hyper;
extern crate reqwest;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate contributors;

use diesel::prelude::*;

use hyper::{Get, StatusCode};
use hyper::server::{Server, Service, Request, Response};

use handlebars::Handlebars;

use std::collections::BTreeMap;
use std::io::prelude::*;
use std::fs::File;

use serde_json::value::Value;

struct Contributors;

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

                use contributors::schema::commits::dsl::*;
                use contributors::models::Commit;

                let connection = contributors::establish_connection();
                let results = commits.load::<Commit>(&connection)
                    .expect("Error loading commits");

                let authors: Vec<_> = results.into_iter().map(|c| Value::String(c.sha)).collect();

                data.insert("shas".to_string(), Value::Array(authors));

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

