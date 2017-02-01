#[derive(Debug,Identifiable,Queryable,Associations)]
#[has_many(releases)]
pub struct Project {
    pub id: i32,
    pub name: String,
    pub url_path: String,
    pub github_name: String,
}

#[derive(Debug,Identifiable,Queryable,Associations)]
#[belongs_to(Release)]
#[belongs_to(Author)]
pub struct Commit {
    pub id: i32,
    pub sha: String,
    pub release_id: i32,
    pub author_id: i32,
}

#[derive(Debug,Identifiable,Queryable,Associations)]
#[has_many(commits)]
#[belongs_to(Project)]
pub struct Release {
    pub id: i32,
    pub version: String,
    pub project_id: i32,
}

#[derive(Debug,Identifiable,Queryable,Associations)]
#[has_many(commits)]
pub struct Author {
    pub id: i32,
    pub name: String,
    pub email: String,
}

use super::schema::projects;

#[derive(Insertable)]
#[table_name="projects"]
pub struct NewProject<'a> {
    pub name: &'a str,
    pub url_path: &'a str,
    pub github_name: &'a str,
}

use super::schema::commits;

#[derive(Insertable)]
#[table_name="commits"]
pub struct NewCommit<'a> {
    pub sha: &'a str,
    pub release_id: i32,
    pub author_id: i32,
}

use super::schema::releases;

#[derive(Insertable)]
#[table_name="releases"]
pub struct NewRelease<'a> {
    pub version: &'a str,
    pub project_id: i32,
}

use super::schema::authors;

#[derive(Insertable)]
#[table_name="authors"]
pub struct NewAuthor<'a> {
    pub name: &'a str,
    pub email: &'a str,
}
