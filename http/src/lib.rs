extern crate futures;
extern crate hyper;
extern crate regex;
extern crate reqwest;

use hyper::StatusCode;
use hyper::header::{ContentType, Location};
use hyper::server::{Http, Service, Request, Response};

use regex::{Regex, Captures};

use std::io::prelude::*;
use std::fs::File;
use std::net::SocketAddr;
use std::path::Path;

pub struct Contributors {
    routes: Vec<Route>,
    catch_all_route: Option<fn(Request) -> ::futures::Finished<Response, hyper::Error>>,
}

pub enum Route {
    Literal {
        path: String,
        handler: fn(Request) -> ::futures::Finished<Response, hyper::Error>,
    },
    Regex {
        regex: Regex,
        handler: fn(&Request, Captures) -> ::futures::Finished<Response, hyper::Error>,
    },
}

impl Route {
    fn matches(&self, path: &str) -> bool {
        match self {
            &Route::Literal { path: ref p, .. } => {
                p == path
            },
            &Route::Regex { ref regex, .. } => {
                regex.is_match(path)
            },
        }
    }

    fn handle(&self, req: Request) -> ::futures::Finished<Response, hyper::Error> {
        match self {
            &Route::Literal { handler, .. } => {
                handler(req)
            },
            &Route::Regex { handler, ref regex } => {
                // i am extremely suspicous of this unwrap
                let captures = regex.captures(req.path()).unwrap();

                handler(&req, captures)
            },
        }
    }
}

impl Contributors {
    pub fn new() -> Contributors {
        Contributors {
            routes: Vec::new(),
            catch_all_route: None,
        }
    }

    pub fn add_route(&mut self, path: &str, handler: fn(Request) -> ::futures::Finished<Response, hyper::Error>) {
        let path = path.to_string();

        self.routes.push(Route::Literal {
            path: path,
            handler: handler,
        });
    }

    pub fn add_regex_route(&mut self, regex: &str, handler: fn(&Request, Captures) -> ::futures::Finished<Response, hyper::Error>) {
        self.routes.push(Route::Regex {
            regex: Regex::new(regex).unwrap(),
            handler: handler,
        });
    }

    pub fn add_catch_all_route(&mut self, f: fn(Request) -> ::futures::Finished<Response, hyper::Error>) {
        self.catch_all_route = Some(f);
    }
}

impl Service for Contributors {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = ::futures::Finished<Response, hyper::Error>;

    fn call(&self, req: Request) -> Self::Future {
        // redirect to ssl
        // from http://jaketrent.com/post/https-redirect-node-heroku/
        if let Some(raw) = req.headers().get_raw("x-forwarded-proto") {
            if raw != &b"https"[..] {
                return ::futures::finished(
                    Response::new()
                    .with_header(Location(format!("https://thanks.rust-lang.org{}", req.path())))
                    .with_status(StatusCode::MovedPermanently)
                );
            }
        }

        // first, we serve static files
        let path = req.path().to_string();

        // ... you trying to do something bad?
        if path.contains("./") || path.contains("../") {
            // GET OUT
            return ::futures::finished(Response::new()
                .with_header(ContentType::html())
                .with_status(StatusCode::NotFound));
        }

        if path.starts_with("/public") && Path::new(&path[1..]).exists() {
            let mut f = File::open(&path[1..]).unwrap();
            let mut source = Vec::new();
            f.read_to_end(&mut source).unwrap();

            return ::futures::finished(Response::new()
              .with_body(source));
        }

        // next, we check routes
        
        for route in &self.routes {
            if route.matches(req.path()) {
                return route.handle(req);
            }
        }

        if let Some(h) = self.catch_all_route {
            return h(req);
        }

        ::futures::finished(Response::new()
                            .with_header(ContentType::html())
                            .with_status(StatusCode::NotFound))
    }
}

pub struct Server;

impl Server {
    pub fn run(&self, addr: &SocketAddr, thanks: Contributors) {
        let a = std::sync::Arc::new(thanks);

        let server = Http::new().bind(addr, move || Ok(a.clone())).unwrap();

        server.run().unwrap();
    }
}
