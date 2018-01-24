use models::{NewProject, Project};

use diesel;
use diesel::result::Error;
use diesel::pg::PgConnection;
use diesel::prelude::*;

pub fn create(conn: &PgConnection, 
              name: &str, 
              url_path: &str, 
              github_name: &str, 
              dir_path: &str) -> Project {
    use schema::projects;

    let new_project = NewProject {
        name,
        url_path,
        github_name,
        dir_path
    };

    diesel::insert_into(projects::table)
        .values(&new_project)
        .get_result(conn)
        .expect("Error saving new project")
}

pub fn delete(conn: &PgConnection, project_name: &str) {
    use schema::projects::dsl::{projects, name};

    diesel::delete(projects.filter(name.eq(project_name)))
        .execute(conn)
        .expect("Error deleting project");
}

pub fn all(conn: &PgConnection) -> Vec<Project> {
    use schema::projects::dsl::*;

    projects.load(conn)
    .expect("Error selecting all projects")
}

pub fn by_name(conn: &PgConnection, project_name: &str) -> Result<Vec<String>, Error> {
    use schema::projects::dsl::*;

    projects.filter(name.eq(project_name))
        .select(dir_path)
        .load(conn)
}

pub fn init(conn: &PgConnection) {
    create(conn, 
           "rust", 
           "https://github.com/rust-lang/rust", 
           "rust", 
           "/repos/github.com/rust-lang/rust");
}

