#[derive(Debug,Queryable)]
pub struct Commit {
    pub sha: String,
    pub author_name: String,
    pub author_email: String,
    pub release_id: Option<i32>,
}

#[derive(Debug,Queryable)]
pub struct Release {
    pub id: i32,
    pub version: String,
}

use super::schema::commits;

#[derive(Insertable)]
#[table_name="commits"]
pub struct NewCommit<'a> {
    pub sha: &'a str,
    pub release_id: Option<i32>,
    pub author_name: &'a str,
    pub author_email: &'a str,
}

use super::schema::releases;

#[derive(Insertable)]
#[table_name="releases"]
pub struct NewRelease<'a> {
    pub version: &'a str,
}
