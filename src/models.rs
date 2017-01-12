#[derive(Queryable)]
pub struct Commit {
    pub id: i32,
    pub sha: String,
    pub author_name: String,
    pub author_email: String,
}
