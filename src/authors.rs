use models::{Author, NewAuthor};
use mailmap::Mailmap;

use diesel::*;
use diesel::pg::PgConnection;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use git2::Repository;
use std::path::Path;

use releases;

// Postgresql won't execute the query if this is much higher
const ITEMS_PER_CHUNK: usize = 30_000;

pub struct AuthorStore<'a> {
    cache: HashMap<(String, String), Author>,
    conn: &'a PgConnection,
    mailmap: Mailmap,
}

impl<'a> AuthorStore<'a> {
    pub fn new(conn: &'a PgConnection, mailmap: Mailmap) -> AuthorStore<'a> {
        AuthorStore {
            cache: HashMap::new(),
            conn: conn,
            mailmap: mailmap,
        }
    }

    pub fn from_file(conn: &'a PgConnection, path: &str) -> AuthorStore<'a> {
        let file_path = Path::new(path).join(".mailmap");

        let contents = {
            if file_path.is_file() {
                let file = File::open(file_path).unwrap();

                let mut buf_reader = BufReader::new(file);
                let mut contents = String::new();
                buf_reader.read_to_string(&mut contents).unwrap();
                contents
            } else {
                "".to_string()
            }
        };

        AuthorStore {
            cache: HashMap::new(),
            conn: conn,
            mailmap: Mailmap::new(contents.as_str()),
        }
    }

    pub fn get(&mut self, author_name: &str, author_email: &str) -> Author {
        let new_author = NewAuthor {
            name: author_name,
            email: author_email,
        };

        let entry = (author_name.to_string(), author_email.to_string());

        if !self.cache.contains_key(&entry) {
            let author = self.find_or_create(&new_author)
                .expect("Could not find or create author")
                .clone();
            self.cache.insert(entry.clone(), author);
        }
        self.cache.get(&entry).unwrap().clone()
    }

    pub fn find_or_create_all<'b>(&mut self, new_authors: Vec<NewAuthor<'b>>) -> Vec<Author> {
        use schema::authors::dsl::*;
        use diesel::expression::dsl::any;
        use diesel::pg::upsert::*;

        let mut found = Vec::new();
        let mut missing = Vec::new();
        let mut missing_names = Vec::new();
        let mut missing_emails = Vec::new();

        // This is more efficient than querying the DB for each author individually
        for author in new_authors.into_iter() {
            let (m_name, m_email) = self.mailmap.map(author.name, author.email);

            match self.cache.get(&(m_name.clone(), m_email.clone())) {
                Some(a) => found.push(a.clone()),
                None => {
                    missing.push(author);
                    missing_names.push(m_name);
                    missing_emails.push(m_email);
                }
            };
        }

        if !missing.is_empty() {
            missing
                .chunks(ITEMS_PER_CHUNK)
                .enumerate()
                .map(|(i, chunk)| {
                    let start = i * ITEMS_PER_CHUNK;
                    let end = start + chunk.len();

                    let the_names = &missing_names[start..end];
                    let the_emails = &missing_emails[start..end];

                    insert_into(authors)
                        .values(chunk)
                        .on_conflict_do_nothing()
                        .execute(self.conn)
                        .unwrap();

                    let db_authors: Vec<Author> = authors
                        .filter(name.eq(any(the_names.clone())))
                        .filter(email.eq(any(the_emails.clone())))
                        .load(self.conn)
                        .unwrap();

                    for new_author in db_authors.into_iter() {
                        found.push(new_author.clone());

                        self.cache.insert(
                            (new_author.name.clone(), new_author.email.clone()),
                            new_author.clone(),
                        );
                    }
                })
                .last();
        }

        found
    }

    pub fn warm_cache(&mut self, repo: &Repository) {
        let commits = releases::get_first_commits(repo, "master");

        let authors: Vec<_> = commits
            .into_iter()
            .map(|id| {
                let c = repo.find_commit(id).unwrap();
                let name: String = c.author().to_owned().name().unwrap().to_owned();
                let email: String = c.author().to_owned().email().unwrap().to_owned();
                (name, email)
            })
            .collect();

        let new_authors: Vec<_> = authors
            .iter()
            .map(|&(ref name, ref email)| NewAuthor {
                name: name.as_str(),
                email: email.as_str(),
            })
            .collect();

        self.find_or_create_all(new_authors);
    }

    pub fn map_author<'b>(&self, author: NewAuthor<'b>) -> (String, String) {
        self.mailmap.map(author.name, author.email)
    }

    pub fn get_mailmap(&self) -> &Mailmap {
        &self.mailmap
    }

    fn find_or_create(&self, new_author: &NewAuthor) -> QueryResult<Author> {
        use schema::authors::dsl::*;
        use diesel::pg::upsert::*;

        let maybe_inserted = insert_into(authors)
            .values(new_author)
            .on_conflict_do_nothing()
            .get_result(self.conn)
            .optional()?;

        if let Some(author) = maybe_inserted {
            return Ok(author);
        }

        authors
            .filter(name.eq(new_author.name))
            .filter(email.eq(new_author.email))
            .first(self.conn)
    }
}
