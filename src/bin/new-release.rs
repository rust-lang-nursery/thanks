extern crate contributors;

extern crate diesel;

use diesel::prelude::*;

use std::process::Command;
use std::env;

fn main() {
    use contributors::schema::releases::dsl::*;
    use contributors::models::Release;

    let connection = contributors::establish_connection();

    let release: Release = releases.order(id.desc()).first(&connection).unwrap();

    let num: u64 = release.version.split(".").nth(1).unwrap().parse().unwrap();
    let new_release = num + 1;
    let new_release_name = format!("1.{}.0", new_release);

    println!("Previous release: {}", release.version);
    println!("Creating new release release: {}", new_release_name);

    if releases.filter(version.eq(&new_release_name)).first::<Release>(&connection).is_ok() {
       panic!("Release {} already exists! Something must be wrong.", new_release_name); 
    }


    let path = env::args().nth(1).unwrap();
    println!("Path to rust repo: {}", path);
    
    let new_release = contributors::create_release(&connection, &new_release_name);
    println!("Created release {}", new_release.version);

    println!("Assigning commits for {}", new_release.version);
    assign_commits(&new_release.version, &release.version);
}

fn assign_commits(release_name: &str, previous_release: &str) {
    let connection = contributors::establish_connection();
    let path = env::args().nth(1).unwrap();

    println!("Assigning commits to release {}", release_name);

    let git_log = Command::new("git")
        .arg("-C")
        .arg(&path)
        .arg("--no-pager")
        .arg("log")
        .arg(r#"--format=%H"#)
        .arg(&format!("{}...{}", previous_release, release_name))
        .output()
        .expect("failed to execute process");

    let log = git_log.stdout;
    let log = String::from_utf8(log).unwrap();

    for sha_name in log.split('\n') {
        // there is a last, blank line
        if sha_name == "" {
            continue;
        }

        println!("Assigning commit {} to release {}", sha_name, release_name);

        //contributors::create_commit(&connection, &sha, &author_name, &author_email, &first_release);
        use contributors::schema::releases::dsl::*;
        use contributors::models::Release;
        use contributors::schema::commits::dsl::*;
        use contributors::models::Commit;

        let the_release = releases.filter(version.eq(&release_name)).first::<Release>(&connection).expect("could not find release");
        
        // did we make this commit earlier? If so, update it. If not, create it
        match commits.filter(sha.eq(&sha_name)).first::<Commit>(&connection) {
            Ok(the_commit) => {
                diesel::update(commits.find(the_commit.id))
                    .set(release_id.eq(the_release.id))
                    .get_result::<Commit>(&connection)
                    .expect(&format!("Unable to update commit {}", the_commit.id));
            },
            Err(_) => {
                let git_log = Command::new("git")
                    .arg("-C")
                    .arg(&path)
                    .arg("--no-pager")
                    .arg("show")
                    .arg(r#"--format=%H %ae %an"#)
                    .arg(&sha_name)
                    .output()
                    .expect("failed to execute process");

                let log = git_log.stdout;
                let log = String::from_utf8(log).unwrap();

                let log_line = log.split('\n').nth(0).unwrap();

                let mut split = log_line.splitn(3, ' ');

                let the_sha = split.next().unwrap();
                let the_author_email = split.next().unwrap();
                let the_author_name = split.next().unwrap();

                println!("Creating commit {} for release {}", the_sha, the_release.version);

                contributors::create_commit(&connection, &the_sha, &the_author_name, &the_author_email, &the_release);
            },
        };

    }
}
