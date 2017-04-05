extern crate futures;
extern crate sparkles;
extern crate thanks;

mod routes;

fn main() {
    let mut server = sparkles::Server::new("templates".to_string());

    server.add_route("/", routes::root);

    let addr = "0.0.0.0:8080".parse().unwrap();
    server.run(&addr);
}
