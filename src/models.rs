#[derive(Debug,Queryable)]
pub struct Commit {
    pub sha: String,
    pub author_name: String,
    pub author_email: String,
}

use super::schema::commits;

#[derive(Insertable)]
#[table_name="commits"]
pub struct NewCommit<'a> {
    pub sha: &'a str,
    pub author_name: &'a str,
    pub author_email: &'a str,
}
