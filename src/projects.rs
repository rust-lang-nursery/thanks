use models::{NewProject, Project};

use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;

pub fn create(conn: &PgConnection, name: &str, url_path: &str, github_name: &str) -> Project {
    use schema::projects;

    let new_project = NewProject {
        name: name,
        url_path: url_path,
        github_name: github_name,
    };

    diesel::insert_into(projects::table)
        .values(&new_project)
        .get_result(conn)
        .expect("Error saving new project")
}
