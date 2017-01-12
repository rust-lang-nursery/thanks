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

                use contributors::schema::releases::dsl::*;
                use contributors::models::Release;

                let connection = contributors::establish_connection();
                let results = releases.load::<Release>(&connection)
                    .expect("Error loading releases");

                let results: Vec<_> = results.into_iter().map(|r| Value::String(r.version)).collect();

                let mut data: BTreeMap<String, Value> = BTreeMap::new();
                data.insert("releases".to_string(), Value::Array(results));

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
                let release_name = path[1..].to_string();
                data.insert("release".to_string(), Value::String(release_name.clone()));

                use contributors::schema::releases::dsl::*;
                use contributors::models::Release;
                use contributors::models::Commit;

                let connection = contributors::establish_connection();

                let release: Release = releases.filter(version.eq(release_name))
                                               .first(&connection).unwrap();

                let results: Vec<Commit> = Commit::belonging_to(&release).load(&connection).unwrap();

                let mut names: Vec<_> = results.into_iter().map(|c| c.author_name).collect();

                // lol i am a bad programmer and am not doing this in the db.
                // it's fine, but this is a good FIXME
                names.sort();
                names.dedup();

                let names: Vec<_> = names.into_iter().map(Value::String).collect();

                data.insert("count".to_string(), Value::U64(names.len() as u64));
                data.insert("names".to_string(), Value::Array(names));

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

