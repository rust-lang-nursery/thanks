extern crate thanks;

extern crate dotenv;
extern crate futures;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate handlebars;
extern crate hyper;
extern crate mime;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate slog_async;

extern crate toml;

#[macro_use]
extern crate lazy_static;

use gotham::http::response::create_response;
use gotham::router::Router;
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
use gotham::state::{FromState, State, client_addr};

use handlebars::Handlebars;

use hyper::server::Response;
use hyper::StatusCode;

//use futures::BoxFuture;

use std::collections;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::sync::Mutex;

use slog::Drain;

use serde_json::value::Value;

lazy_static! {
    /// CACHE is an in-memory store of the repo data, 
    /// so that we don't have to walk the git repos
    /// for every request.
    pub static ref CACHE: Mutex<HashMap<String, Project>> = Mutex::new({
        HashMap::new()
    });

    // A list of (project name, repo path) tuples.
    pub static ref PROJECTS: Mutex<Vec<thanks::models::Project>> = Mutex::new({
        vec!()
    });

    // TEMPLATES stores the HTML templates.
    // TODO(rm): pass relevant template to handler when building router.
    pub static ref TEMPLATES: Mutex<Handlebars> = Mutex::new({
        Handlebars::new()
    });
}

/// Project is an in-memory store of the releases, commits and authors for a project.
pub struct Project {
    // TODO(rm): review usage of this struct. 
    name: String,
    tags: Vec<Value>,
    releases: Vec<Release>,
    all_time: Vec<Value>,
}

/// Release is a tag name and the authors of commits for that tag.
pub struct Release {
    name: String,
    authors: Vec<Value>,
}

fn main() {
    dotenv::dotenv().ok();

    // Setup logging.
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let log = slog::Logger::root(drain, o!("version" => env!("CARGO_PKG_VERSION")));

    let addr: String = 
        format!("0.0.0.0:{}",
                env::args().nth(1).unwrap_or_else(|| String::from("1337")))
        .parse()
        .unwrap();

    // TODO(rm): include multiple repos in stats.

    // Setup db connection.
    let conn = thanks::establish_connection();
    let projects = thanks::projects::all(&conn);

    // Load data into memory.
    info!(log, "Warming cache");
    let new_cache = warm_cache(projects);
    *CACHE.lock().unwrap() = new_cache;
    info!(log, "Finished warming cache");

    load_templates();

    // Gotham server:
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router())
}

/// `load_templates` reads all of the handlebars template files into memory.
fn load_templates() {
    let template_root = "templates";

    let mut handlebars = TEMPLATES.lock().unwrap();

    for entry in fs::read_dir(&template_root).unwrap() {
        let entry = entry.unwrap();

        // Skip entries which are not templates.
        if let Ok(file_type) = entry.file_type() {
            if !file_type.is_file() {
                continue;
            }
        } else {
            continue;
        }

        if let Ok(file_name) = entry.file_name().into_string() {
            if file_name.starts_with('.') {
                continue;
            }
        } else {
            continue;
        }

        if let Ok(file_name) = entry.file_name().into_string() {
            if ! file_name.ends_with(".hbs") {
                continue;
            }
        } else {
            continue;
        }

        let path = entry.path();
        let name = path.file_stem().unwrap().to_str().unwrap();

        handlebars
            .register_template_file(name, &path)
            .ok()
            .unwrap();
    }
}

/// `router` creates a Gotham router.
fn router() -> Router {
    build_simple_router(|route| {
        route.get("/:project").with_path_extractor::<RootPath>().to(root);
        route.get("/about").to(about);
        route.get("/:project/all-time").with_path_extractor::<AllTimePath>().to(cached_all_time);
        route.get("/:project/:version").with_path_extractor::<ReleasePath>().to(cached_release);
        route.get("/styles/:name").with_path_extractor::<ResourcePath>().to(public_styles);
        route.get("/fonts/:name").with_path_extractor::<ResourcePath>().to(public_fonts);
        route.get("/scripts/:name").with_path_extractor::<ResourcePath>().to(public_scripts);
        route.get("/images/:name").with_path_extractor::<ResourcePath>().to(public_images);
        route.get("/reload").to(reload);
    })
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct RootPath {
    project: String,
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct AllTimePath {
    project: String,
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct ReleasePath {
    project: String,
    version: String,
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct ResourcePath {
    name: String,
}

fn public_styles(state: State) -> (State, Response) {
    public_resources(state, "styles", mime::TEXT_CSS)
}

fn public_fonts(state: State) -> (State, Response) {
    // TODO(rm): serve fonts.
    let m: mime::Mime = "application/x-font-ttf".parse().unwrap();
    let (state, res) = public_resources(state, "fonts", m);
    (state, res)
}

fn public_images(state: State) -> (State, Response) {
    public_resources(state, "images", mime::IMAGE_PNG)
}

fn public_scripts(state: State) -> (State, Response) {
    public_resources(state, "scripts", mime::TEXT_JAVASCRIPT)
}

fn public_resources(state: State, sub_dir: &str, mime_type: mime::Mime) -> (State, Response) {
    let res = {
        let resource = ResourcePath::borrow_from(&state);
        
        let file_path = format!("public/{}/{}", sub_dir, resource.name);

        match File::open(file_path) {
            Ok(mut file) => {
                let mut contents: Vec<u8> = vec!();
                match file.read_to_end(&mut contents) {
                    Ok(_num_bytes_read) => {
                        create_response(
                            &state, 
                            StatusCode::Ok, 
                            Some(
                                (contents, mime_type)
                            ),
                         )
                    },
                    Err(e) => {
                        create_response(
                            &state, 
                            StatusCode::NotFound, 
                            Some(
                                (format!("no resource found: {}", e).as_bytes().to_vec(), 
                                mime::TEXT_PLAIN)
                            ),
                        )
                    },
                }
            },
            Err(e) => {
                create_response(
                    &state, 
                    StatusCode::NotFound, 
                    Some(
                        (format!("no resource found: {}", e).as_bytes().to_vec(), 
                        mime::TEXT_PLAIN)
                    ),
                 )
            },
        }

    };

    (state, res)
}

/// `warm_cache` loads the git repo data into a `HashMap`.
fn warm_cache(repos: Vec<thanks::models::Project>) -> HashMap<String, Project> {
    let mut tmp_cache: HashMap<String, Project> = HashMap::new();
    
    for project in repos {
        // Get all_time.
        let all_time_scores: Vec<Value> = thanks::scores(&project.dir_path);

        // Get all_release_tags.
        let tags: Vec<Value> = thanks::releases::all(&project.dir_path);

        // Get all_releases_authors.
        let mut all_releases: Vec<Release> = vec!();
        for i in &tags {
            println!("{}", i);
            match thanks::releases::contributors(&tags, &project.dir_path, i.as_str().unwrap()) {
                Some(names) => all_releases.push(
                    Release {
                        name: i.as_str().unwrap().to_string(), 
                        authors: names,
                    }
                ),
                None => panic!("failed warming cache with release: {}", i.to_string()),
            };
        }

        let project = Project{
            name: project.name.clone(),
            tags: tags,
            releases: all_releases,
            all_time: all_time_scores,
        };

        let project_name = project.name.to_owned();
        tmp_cache.insert(project_name, project);
    }

    tmp_cache
}

/// `reload` is an endpoint which reloads the cache from the Git repositories.
fn reload(state: State) -> (State, Response) {
    // Only allow reloads from localhost.
    let is_admin = match client_addr(&state) {
        Some(v) => v.ip().is_loopback() ,
        None    => false,
    };

    if !is_admin {
        let res = create_response(
            &state,
            StatusCode::NoContent,
            Some(("No content.".to_string().into_bytes(), mime::TEXT_HTML)),
        );

        return (state, res)
    }

    let repos = PROJECTS.lock().unwrap();
    let new_cache = warm_cache(repos.to_vec());
    
    // Swap out the old cache for the new cache.
    *CACHE.lock().unwrap() = new_cache;

    let res = create_response(
        &state,
        StatusCode::Ok,
        Some(("Reloaded.".to_string().into_bytes(), mime::TEXT_HTML)),
    );

    (state, res)
}

fn root(state: State) -> (State, Response) {
    let mut data: collections::BTreeMap<String, Value> = collections::BTreeMap::new();

    data.insert("maintenance".to_string(), Value::Bool(thanks::in_maintenance()));

    // Get the releases for the specified project.
    let mut project_name = RootPath::borrow_from(&state).project.to_string();
    if project_name.is_empty() {
        project_name = "rust".to_string();
    }

    let conn = thanks::establish_connection();
    let repo_path = match thanks::projects::by_name(&conn, &project_name) {
        Ok(v)   => {
           match v.get(0) {
               Some(s) => s.clone(),
               None => {
                    let res = create_response(
                        &state, 
                        StatusCode::NotFound, 
                        Some(
                            (b"repo path not found".to_vec(), 
                             mime::TEXT_PLAIN)
                            ),
                        );

                    return (state, res)
               },
           }
        },
        Err(e)  => {
            let res = create_response(
                &state, 
                StatusCode::NotFound, 
                Some(
                    (format!("error looking for repo path: {}", e).as_bytes().to_vec(), 
                     mime::TEXT_PLAIN)
                    ),
                );

            return (state, res)
        },
    };


    data.insert(
        "releases".to_string(), 
        Value::Array(thanks::releases::all(&repo_path)));
    
    let handlebars = TEMPLATES.lock().unwrap();

    let res = create_response(
        &state,
        StatusCode::Ok,
        Some((handlebars.render("index", &data).unwrap().into_bytes(), mime::TEXT_HTML)),
    );

    (state, res)
}

fn about(state: State) -> (State, Response) {
    let mut data: collections::BTreeMap<String, Value> = collections::BTreeMap::new();

    data.insert("maintenance".to_string(), Value::Bool(thanks::in_maintenance()));

    let handlebars = TEMPLATES.lock().unwrap();

    let res = create_response(
        &state,
        StatusCode::Ok,
        Some((handlebars.render("about", &data).unwrap().into_bytes(), mime::TEXT_HTML)),
    );
    
    (state, res)
}

fn cached_all_time(state: State) -> (State, Response) {
    let res = {
        let all_time_path = AllTimePath::borrow_from(&state);

        let cache = CACHE.lock().unwrap();
        let proj = cache.get(all_time_path.project.as_str()).unwrap();

        let mut data: collections::BTreeMap<String, Value> = collections::BTreeMap::new();

        data.insert("maintenance".to_string(), Value::Bool(thanks::in_maintenance()));

        let scores = &proj.all_time;

        data.insert("release".to_string(), Value::String(String::from("all-time")));
        data.insert("count".to_string(), Value::Number((scores.len() as u64).into()));
        data.insert("scores".to_string(), Value::Array(scores.to_vec()));

        let handlebars = TEMPLATES.lock().unwrap();

        create_response(
            &state,
            StatusCode::Ok,
            Some((handlebars.render("all-time", &data).unwrap().into_bytes(), mime::TEXT_HTML)),
        )
    };

    (state, res)
}

fn cached_release(state: State) -> (State, Response) {
    let res = {
        let release = ReleasePath::borrow_from(&state);

        let cache = CACHE.lock().unwrap();
        let proj = cache.get(release.project.as_str()).unwrap();

        let mut data: collections::BTreeMap<String, Value> = collections::BTreeMap::new();

        data.insert("maintenance".to_string(), Value::Bool(thanks::in_maintenance()));

        let release_name = release.version.as_str();

        data.insert("release".to_string(), Value::String(release_name.to_string()));
        data.insert("link".to_string(), Value::String(release_name.to_string()));

        let mut names: Vec<Value> = vec!();
        for i in &proj.releases {
            if i.name == release_name {
                names = i.authors.to_owned();
                break;
            }
        }

        data.insert("count".to_string(), Value::Number((names.len() as u64).into()));
        data.insert("names".to_string(), Value::Array(names));
        data.insert("link".to_string(), 
                    Value::String(changelog_link(release_name).to_string()));

        let handlebars = TEMPLATES.lock().unwrap();

        create_response(
            &state,
            StatusCode::Ok,
            Some((handlebars.render("release", &data).unwrap().into_bytes(), mime::TEXT_HTML)),
        )
    };

    (state, res)
}

/// `load_projects` reads a list of project repository paths from a toml file.
pub fn load_projects(path: &str) -> Vec<String> {
    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(e)   => panic!("error opening projects file: {}", e),
    };

    let mut projects_toml = String::new();

    match file.read_to_string(&mut projects_toml) {
        Ok(size) => size,
        Err(e) => panic!("failed reading projects file: {}", e),
    };

    let hashmap: HashMap<String, Vec<String>> = 
        match toml::from_str(&projects_toml) {
            Ok(hashmap) => hashmap,
            Err(e)      => panic!("failed deserializing projects list: {}", e),
        };

    let list: Vec<String> = match hashmap.get("projects") {
        Some(v)  => v.to_owned(),
        None     => panic!("projects list not found in file"),
    };

    list
}

// TODO(rm): move to db.
fn changelog_link(version: &str) -> &str {
    match version {
        "master"        => "https://github.com/rust-lang/rust/commits/master",
        "0.1"           => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-01--2012-01-20",
        "0.2"           => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-02--2012-03-29",
        "0.3"           => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-03--2012-07-12",
        "0.4"           => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-04-2012-10-15",
        "0.5"           => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-05-2012-12-21",
        "0.6"           => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-06-2013-04-03",
        "0.7"           => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-07-2013-07-03",
        "0.8"           => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-08-2013-09-26",
        "0.9"           => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-09-2014-01-09",
        "0.10"          => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-010-2014-04-03",
        "0.11.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-0110-2014-07-02",
        "0.12.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-0120-2014-10-09",
        "1.0.0-alpha"   => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-100-alpha-2015-01-09",
        "1.0.0-alpha.2" => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-100-alpha2-2015-02-20",
        "1.0.0-beta"    => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#",
        "1.0.0"         => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-100-2015-05-15",
        "1.1.0"         => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-110-2015-06-25",
        "1.2.0"         => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-120-2015-08-07",
        "1.3.0"         => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-130-2015-09-17",
        "1.4.0"         => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-140-2015-10-29",
        "1.5.0"         => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-150-2015-12-10",
        "1.6.0"         => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-160-2016-01-21",
        "1.7.0"         => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-170-2016-03-03",
        "1.8.0"         => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-180-2016-04-14",
        "1.9.0"         => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-190-2016-05-26",
        "1.10.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1100-2016-07-07",
        "1.11.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1110-2016-08-18",
        "1.12.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1120-2016-09-29",
        "1.12.1"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1121-2016-10-20",
        "1.13.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1130-2016-11-10",
        "1.14.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1140-2016-12-22",
        "1.15.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1150-2017-02-02",
        "1.15.1"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1151-2017-02-09",
        "1.16.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1160-2017-03-16",
        "1.17.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1170-2017-04-27",
        "1.18.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1180-2017-06-08",
        "1.19.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1190-2017-07-20",
        "1.20.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1200-2017-08-31",
        "1.21.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1210-2017-10-12",
        "1.22.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1220-2017-11-22",
        "1.22.1"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1221-2017-11-22",
        "1.23.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1230-2018-01-04",
        "1.24.0"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1240-2018-02-15",
        "1.24.1"        => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1241-2018-03-02",
        _               => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#",
    }
}
