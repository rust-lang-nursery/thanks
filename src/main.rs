#[macro_use]
extern crate diesel;

extern crate dotenv;

extern crate futures;

extern crate handlebars;

extern crate hyper;
extern crate reqwest;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate itertools;

extern crate contributors;

use diesel::prelude::*;

use hyper::{Get, StatusCode};
use hyper::header::ContentType;
use hyper::server::{Server, Service, Request, Response};

use handlebars::Handlebars;

use std::collections::BTreeMap;
use std::env;
use std::io::prelude::*;
use std::fs::File;

use serde_json::value::Value;

use itertools::Itertools;

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
                let results = releases.filter(version.ne("master"))
                    .load::<Release>(&connection)
                    .expect("Error loading releases");

                let results: Vec<_> = results.into_iter()
                    .rev()
                    .map(|r| Value::String(r.version))
                    .collect();

                let mut data: BTreeMap<String, Value> = BTreeMap::new();

                data.insert("releases".to_string(), Value::Array(results));

                Response::new()
                    .with_header(ContentType::html())
                    .with_body(handlebars.template_render(&source, &data).unwrap())
            },
            (&Get, Some(path)) => {
                let handlebars = Handlebars::new();

                let mut source = String::new();

                let mut data: BTreeMap<String, Value> = BTreeMap::new();

                // strip the leading `/` lol
                let release_name = path[1..].to_string();

                data.insert("release".to_string(), Value::String(release_name.clone()));

                if release_name == "all-time" {
                    let mut f = File::open("templates/all-time.hbs").unwrap();

                    f.read_to_string(&mut source).unwrap();

                    use contributors::schema::commits::dsl::*;
                    use contributors::models::Commit;

                    let connection = contributors::establish_connection();

                    // It's possible to do this in Postgres with
                    // SELECT author_name, COUNT(author_name) as commit_count FROM commits GROUP BY author_name ORDER BY commit_count DESC;
                    // but it doesn't look like Diesel supports that
                    let mut results: Vec<Commit> = commits.load(&connection).unwrap();

                    results.sort_by_key(|c| c.author_name.clone());
                    let grouped = results.iter().group_by(|c| c.author_name.clone());
                    let mut scores: Vec<_> = grouped.into_iter()
                      .map(|(author, by_author)| (author, by_author.count())).collect();

                    scores.sort_by_key(|&(_, score)| score);
                    scores.reverse();

                    let scores: Vec<_> = scores.into_iter().map(|(author, score)| {
                        let mut json_score: BTreeMap<String, Value> = BTreeMap::new();
                        json_score.insert("author".to_string(), Value::String(author));
                        json_score.insert("commits".to_string(), Value::U64(score as u64));

                        Value::Object(json_score)
                    }).collect();

                    data.insert("count".to_string(), Value::U64(scores.len() as u64));
                    data.insert("scores".to_string(), Value::Array(scores));
                } else {
                    let mut f = File::open("templates/release.hbs").unwrap();
                    f.read_to_string(&mut source).unwrap();

                    use contributors::schema::releases::dsl::*;
                    use contributors::schema::commits::dsl::*;
                    use contributors::models::Release;
                    use contributors::models::Commit;

                    let connection = contributors::establish_connection();

                    let release: Release = match releases.filter(version.eq(release_name))
                                                   .first(&connection) {
                        Ok(release) => release,
                        Err(_) => {
                            return ::futures::finished(Response::new()
                                .with_status(StatusCode::NotFound));
                        },
                    };

                    // it'd be better to do this in the db
                    // but Postgres doesn't do Unicode collation correctly on OSX
                    // http://postgresql.nabble.com/Collate-order-on-Mac-OS-X-text-with-diacritics-in-UTF-8-td1912473.html
                    let mut names: Vec<String> = Commit::belonging_to(&release)
                        .select(author_name).distinct().load(&connection).unwrap();

                    contributors::inaccurate_sort(&mut names);

                    let names: Vec<_> = names.into_iter().map(Value::String).collect();

                    data.insert("count".to_string(), Value::U64(names.len() as u64));
                    data.insert("names".to_string(), Value::Array(names));
                }

                Response::new()
                    .with_header(ContentType::html())
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
    dotenv::dotenv().ok();

    let addr = format!("0.0.0.0:{}", env::args().nth(1).unwrap_or(String::from("1337"))).parse().unwrap();

    let (listening, server) = Server::standalone(|tokio| {
        Server::http(&addr, tokio)?
            .handle(|| Ok(Contributors), tokio)
    }).unwrap();
    println!("Listening on http://{}", listening);
    server.run();
}

