use diesel;
use diesel::prelude::*;

use std::process::Command;

use slog::Logger;

pub fn assign_commits(log: &Logger, release_name: &str, previous_release: &str, release_project_id: i32, path: &str) {
    // Could take the connection as a parameter, as problably
    // it's already established somewhere...
    let connection = ::establish_connection();

    info!(log, "Assigning commits to release {}", release_name);

    let git_log = Command::new("git")
        .arg("-C")
        .arg(&path)
        .arg("--no-pager")
        .arg("log")
        .arg("--use-mailmap")
        .arg(r#"--format=%H"#)
        .arg(&format!("{}...{}", previous_release, release_name))
        .output()
        .expect("failed to execute process");

    let git_log = git_log.stdout;
    let git_log = String::from_utf8(git_log).unwrap();

    for sha_name in git_log.split('\n') {
        // there is a last, blank line
        if sha_name == "" {
            continue;
        }

        info!(log, "Assigning commit {} to release {}", sha_name, release_name);

        use schema::releases::dsl::*;
        use models::Release;
        use schema::commits::dsl::*;
        use models::Commit;

        let the_release = releases
            .filter(version.eq(&release_name))
            .filter(project_id.eq(release_project_id))
            .first::<Release>(&connection)
            .expect("could not find release");

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

                let git_log = git_log.stdout;
                let git_log = String::from_utf8(git_log).unwrap();

                let log_line = git_log.split('\n').nth(0).unwrap();

                let mut split = log_line.splitn(3, ' ');

                let the_sha = split.next().unwrap();
                let the_author_email = split.next().unwrap();
                let the_author_name = split.next().unwrap();

                info!(log, "Creating commit {} for release {}", the_sha, the_release.version);

                ::create_commit(&connection, &the_sha, &the_author_name, &the_author_email, &the_release);
            },
        };
    }
}

