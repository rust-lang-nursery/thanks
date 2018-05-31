extern crate thanks;

extern crate diesel;

extern crate dotenv;

extern crate futures;

extern crate handlebars;

extern crate reqwest;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate slog;
extern crate slog_term;

extern crate clap;
extern crate git2;

use diesel::prelude::*;
use clap::{App, Arg};
use slog::DrainExt;

use git2::Repository;

fn main() {
    let matches = App::new("populate")
        .about("initialize the database")
        .arg(
            Arg::with_name("filepath")
                .short("p")
                .long("path")
                .help("filepath of the source code")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("url_path")
                .short("u")
                .long("url")
                .help("url path for this project")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("name")
                .short("n")
                .long("name")
                .help("name of the project")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("github_name")
                .short("g")
                .long("github")
                .help("GitHub link of the project")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let log = slog::Logger::root(
        slog_term::streamer().full().build().fuse(),
        o!("version" => env!("CARGO_PKG_VERSION")),
    );

    let connection = thanks::establish_connection();

    // get name
    let project_name = matches.value_of("name").unwrap();
    info!(log, "Project name: {}", project_name);

    // check that we have no releases for given project
    {
        use thanks::models::Release;
        use thanks::schema::projects::dsl::*;
        use thanks::models::Project;

        if let Ok(project) = projects
            .filter(name.eq(project_name))
            .load::<Project>(&connection)
        {
            if let Ok(count) = Release::belonging_to(&project)
                .count()
                .first::<i64>(&connection)
            {
                if count > 0 {
                    panic!("you have releases in here already");
                }
            }
        }
    }

    // check that we have no commits
    // if there are no releases then there should be no commits as well
    // so we may skip this check
    // I consider changing release_id to NOT NULL since we assign commit
    // to the first release on creation

    // get path to git repo
    let path = matches.value_of("filepath").unwrap();
    info!(log, "Path to project's repo: {}", path);

    // get url path
    let url_path = matches.value_of("url_path").unwrap();
    info!(log, "URL path: {}", url_path);

    // get github name
    let github_name = matches.value_of("github_name").unwrap();
    info!(log, "GitHub name: {}", github_name);

    // create project
    let project = thanks::projects::create(&connection, project_name, url_path, github_name);

    // Create releases
    let releases = [
        // version, previous version, link
        ("0.2", "0.1", changelog_link("0.2")),
        ("0.3", "0.2", changelog_link("0.3")),
        ("0.4", "0.3", changelog_link("0.4")),
        ("0.5", "0.4", changelog_link("0.5")),
        ("0.6", "0.5", changelog_link("0.6")),
        ("0.7", "0.6", changelog_link("0.7")),
        ("0.8", "0.7", changelog_link("0.8")),
        ("0.9", "0.8", changelog_link("0.9")),
        ("0.10", "0.9", changelog_link("0.10")),
        ("0.11.0", "0.10", changelog_link("0.11.0")),
        ("0.12.0", "0.11.0", changelog_link("0.12.0")),
        ("1.0.0-alpha", "0.12.0", changelog_link("1.0.0-alpha")),
        (
            "1.0.0-alpha.2",
            "1.0.0-alpha",
            changelog_link("1.0.0-alpha.2"),
        ),
        ("1.0.0-beta", "1.0.0-alpha.2", changelog_link("1.0.0-beta")),
        ("1.0.0", "1.0.0-beta", changelog_link("1.0.0")),
        ("1.1.0", "1.0.0", changelog_link("1.1.0")),
        ("1.2.0", "1.1.0", changelog_link("1.2.0")),
        ("1.3.0", "1.2.0", changelog_link("1.3.0")),
        ("1.4.0", "1.3.0", changelog_link("1.4.0")),
        ("1.5.0", "1.4.0", changelog_link("1.5.0")),
        ("1.6.0", "1.5.0", changelog_link("1.6.0")),
        ("1.7.0", "1.6.0", changelog_link("1.7.0")),
        ("1.8.0", "1.7.0", changelog_link("1.8.0")),
        ("1.9.0", "1.8.0", changelog_link("1.9.0")),
        ("1.10.0", "1.9.0", changelog_link("1.10.0")),
        ("1.11.0", "1.10.0", changelog_link("1.11.0")),
        ("1.12.0", "1.11.0", changelog_link("1.12.0")),
        ("1.12.1", "1.12.0", changelog_link("1.12.1")),
        ("1.13.0", "1.12.0", changelog_link("1.13.0")),
        ("1.14.0", "1.13.0", changelog_link("1.14.0")),
        ("1.15.0", "1.14.0", changelog_link("1.15.0")),
        ("1.15.1", "1.15.0", changelog_link("1.15.1")),
        ("1.16.0", "1.15.0", changelog_link("1.16.0")),
    ];

    // create 0.1, which isn't in the loop because it will have everything assigned
    // to it by default
    thanks::releases::create(&connection, "0.1", project.id, true, changelog_link("0.1"));

    for &(release, _, link) in releases.iter() {
        thanks::releases::create(&connection, release, project.id, true, link);
    }

    // And create the release for all commits that are not released yet
    thanks::releases::create(
        &connection,
        "master",
        project.id,
        true,
        changelog_link("master"),
    );

    let repo = Repository::open(path).unwrap();

    let mut lookup = thanks::authors::AuthorStore::from_file(&connection, path);
    lookup.warm_cache(&repo);

    // assign first release
    thanks::releases::assign_commits(
        &log,
        &repo,
        &mut lookup,
        "0.1",
        thanks::releases::get_first_commits(&repo, "0.1"),
        project.id,
    );

    // assign commits to their release
    for &(release, previous, _) in releases.iter() {
        thanks::releases::assign_commits(
            &log,
            &repo,
            &mut lookup,
            release,
            thanks::releases::get_commits(&repo, release, previous),
            project.id,
        );
    }

    // assign master
    let last = releases.last().unwrap().0;
    thanks::releases::assign_commits(
        &log,
        &repo,
        &mut lookup,
        "master",
        thanks::releases::get_commits(&repo, "master", last),
        project.id,
    );

    info!(log, "Done!");
}

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
        _               => "https://github.com/rust-lang/rust/blob/master/RELEASES.md#",
    }
}
