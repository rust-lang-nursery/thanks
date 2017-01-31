extern crate futures;

extern crate hyper;
extern crate reqwest;

use hyper::StatusCode;
use hyper::header::{ContentType, Location};
use hyper::server::{Http, Service, Request, Response};

use std::io::prelude::*;
use std::fs::File;
use std::net::SocketAddr;
use std::path::Path;

pub struct Contributors {
    routes: Vec<Route>,
    catch_all_route: Option<fn(Request) -> ::futures::Finished<Response, hyper::Error>>,
}

pub enum RouteKind {
    Literal(String),
}

pub struct Route {
    kind: RouteKind,
    handler: fn(Request) -> ::futures::Finished<Response, hyper::Error>,
}

impl Route {
    fn matches(&self, path: &str) -> bool {
        match self.kind {
            RouteKind::Literal(ref s) => {
                s == path
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

    pub fn add_route(&mut self, path: &str, f: fn(Request) -> ::futures::Finished<Response, hyper::Error>) {
        let route = Route {
            kind: RouteKind::Literal(path.to_string()),
            handler: f,
        };

        self.routes.push(route);
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
                return (route.handler)(req);
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
    pub fn run(&self, addr: &SocketAddr, contributors: Contributors) {
        let a = std::sync::Arc::new(contributors);

        let server = Http::new().bind(addr, move || Ok(a.clone())).unwrap();

        server.run().unwrap();
    }
}
