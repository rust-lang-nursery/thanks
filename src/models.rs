#[derive(Queryable)]
pub struct Commit {
    pub sha: String,
    pub author_name: String,
    pub author_email: String,
}
