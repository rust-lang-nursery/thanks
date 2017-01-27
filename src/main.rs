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

use diesel::prelude::*;

use hyper::StatusCode;
use hyper::header::ContentType;
use hyper::server::Response;

use handlebars::Handlebars;

use std::collections::BTreeMap;
use std::env;
use std::io::prelude::*;
use std::fs::File;

use serde_json::value::Value;

fn main() {
    dotenv::dotenv().ok();

    let addr = format!("0.0.0.0:{}", env::args().nth(1).unwrap_or(String::from("1337"))).parse().unwrap();

    let server = http::Server;
    let mut contributors = http::Contributors::new();

    contributors.add_route("/", |_| {
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

        ::futures::finished(Response::new()
            .with_header(ContentType::html())
            .with_body(handlebars.template_render(&source, &data).unwrap())
        )
    });

    // * is the catch-all route
    contributors.add_route("*", |req| {
        let path = req.path();

        let handlebars = Handlebars::new();

        let mut source = String::new();

        let mut data: BTreeMap<String, Value> = BTreeMap::new();

        // strip the leading `/` lol
        let release_name = path[1..].to_string();

        data.insert("release".to_string(), Value::String(release_name.clone()));

        if release_name == "all-time" {
            println!("all-time arm\npath: {}", path);
            let mut f = File::open("templates/all-time.hbs").unwrap();

            f.read_to_string(&mut source).unwrap();

            use contributors::schema::commits::dsl::*;
            use diesel::expression::dsl::sql;
            use diesel::types::BigInt;

            let connection = contributors::establish_connection();

            let scores: Vec<_> =
                commits
                .select((author_name, sql::<BigInt>("COUNT(author_name) AS author_count")))
                .group_by(author_name)
                .order(sql::<BigInt>("author_count").desc())
                .load(&connection)
                .unwrap();

            let scores: Vec<_> = scores.into_iter().map(|(author, score)| {
                let mut json_score: BTreeMap<String, Value> = BTreeMap::new();
                json_score.insert("author".to_string(), Value::String(author));
                json_score.insert("commits".to_string(), Value::I64(score));

                Value::Object(json_score)
            }).collect();

            data.insert("count".to_string(), Value::U64(scores.len() as u64));
            data.insert("scores".to_string(), Value::Array(scores));
        // serve files in a public directory statically
        } else {
            println!("releases arm\npath: {}", path);
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

        ::futures::finished(Response::new()
            .with_header(ContentType::html())
            .with_body(handlebars.template_render(&source, &data).unwrap())
        )
    });

    println!("Starting server, listening on http://{}", addr);

    server.run(&addr, contributors);
}
