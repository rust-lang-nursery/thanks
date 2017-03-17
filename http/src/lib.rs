extern crate futures;
extern crate hyper;
extern crate regex;
extern crate reqwest;
extern crate serde_json;
extern crate handlebars;

use hyper::StatusCode;
use hyper::header::{ContentType, Location};
use hyper::server::{Http, Service};

use handlebars::Handlebars;

use regex::{Regex, Captures};

use std::io::prelude::*;
use std::fs::File;
use std::net::SocketAddr;
use std::path::Path;

use serde_json::value::Value;
// Rename type for crate
type BTreeMap = std::collections::BTreeMap<String, Value>;

pub struct Request {
    request: hyper::server::Request,
}

pub enum Response {
    Success {
        data: BTreeMap,
        template: String,
    },
    NotFound,
}

pub struct Server {
    routes: Vec<Route>,
    catch_all_route: Option<fn(Request) -> Response>,
    template_root: String,
}

pub enum Route {
    Literal {
        path: String,
        handler: fn(Request) -> Response,
    },
    Regex {
        regex: Regex,
        handler: fn(&Request, Captures) -> Response,
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

    fn handle(&self, req: Request) -> Response {
        match self {
            &Route::Literal { handler, .. } => {
                handler(req)
            },
            &Route::Regex { handler, ref regex } => {
                // i am extremely suspicous of this unwrap
                let captures = regex.captures(req.request.path()).unwrap();

                handler(&req, captures)
            },
        }
    }
}

impl Server {
    pub fn new(template_root: String) -> Server {
        Server {
            routes: Vec::new(),
            catch_all_route: None,
            template_root: template_root,
        }
    }

    pub fn add_route(&mut self, path: &str, handler: fn(Request) -> Response) {
        let path = path.to_string();

        self.routes.push(Route::Literal {
            path: path,
            handler: handler,
        });
    }

    pub fn add_regex_route(&mut self, regex: &str, handler: fn(&Request, Captures) -> Response) {
        self.routes.push(Route::Regex {
            regex: Regex::new(regex).unwrap(),
            handler: handler,
        });
    }

    pub fn add_catch_all_route(&mut self, f: fn(Request) -> Response) {
        self.catch_all_route = Some(f);
    }

    fn build_template(&self, data: &BTreeMap, template_path: &str) -> String {
        let mut handlebars = Handlebars::new();
        // Render the partials
        handlebars.register_template_file("container", &Path::new(&format!("{}/container.hbs", self.template_root)))
            .ok()
            .unwrap();
        handlebars.register_template_file("index", &Path::new(&format!("{}/{}", self.template_root, template_path)))
            .ok()
            .unwrap();
        let mut data = data.clone();
        // Add name of the container to be loaded (just a constant for now)
        data.insert("parent".to_string(), Value::String("container".to_string()));

        // That's all we need to build this thing
        handlebars.render("index", &data).unwrap()
    }
}

impl Service for Server {
    type Request = hyper::server::Request;
    type Response = hyper::server::Response;
    type Error = hyper::Error;
    type Future = ::futures::Finished<hyper::server::Response, hyper::Error>;

    fn call(&self, req: hyper::server::Request) -> Self::Future {
        // redirect to ssl
        // from http://jaketrent.com/post/https-redirect-node-heroku/
        if let Some(raw) = req.headers().get_raw("x-forwarded-proto") {
            if raw != &b"https"[..] {
                return ::futures::finished(
                    hyper::server::Response::new()
                    .with_header(Location(format!("https://thanks.rust-lang.org{}", req.path())))
                    .with_status(StatusCode::MovedPermanently)
                );
            }
        }

        // first, we serve static files
        let fs_path = format!("public{}", req.path());

        // ... you trying to do something bad?
        if fs_path.contains("./") || fs_path.contains("../") {
            // GET OUT
            return ::futures::finished(hyper::server::Response::new()
                .with_header(ContentType::html())
                .with_status(StatusCode::NotFound));
        }

        if Path::new(&fs_path).is_file() {
            let mut f = File::open(&fs_path).unwrap();
            let mut source = Vec::new();
            f.read_to_end(&mut source).unwrap();

            return ::futures::finished(hyper::server::Response::new()
              .with_body(source));
        }

        // next, we check routes
        
        for route in &self.routes {
            if route.matches(req.path()) {
                let r = Request {
                    request: req,
                };
                let response = route.handle(r);

                match response {
                    Response::Success { data, template } => {
                        let body = self.build_template(&data, &template);

                        return ::futures::finished(hyper::server::Response::new()
                            .with_header(ContentType::html())
                            .with_body(body));
                    }
                    Response::NotFound => {
                        return ::futures::finished(hyper::server::Response::new().with_status(StatusCode::NotFound));
                    }
                }
            }
        }

        if let Some(h) = self.catch_all_route {
            let r = Request {
                request: req,
            };
            let response = h(r);

            match response {
                Response::Success { data, template } => {
                    let body = self.build_template(&data, &template);

                    return ::futures::finished(hyper::server::Response::new()
                        .with_header(ContentType::html())
                        .with_body(body));
                }
                Response::NotFound => {
                    return ::futures::finished(hyper::server::Response::new().with_status(StatusCode::NotFound));
                }
            }
        }

        ::futures::finished(hyper::server::Response::new()
                            .with_header(ContentType::html())
                            .with_status(StatusCode::NotFound))
    }
}

impl Server {
    pub fn run(self, addr: &SocketAddr) {
        let a = std::sync::Arc::new(self);

        let server = Http::new().bind(addr, move || Ok(a.clone())).unwrap();

        server.run().unwrap();
    }
}