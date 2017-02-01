use models::{Author, NewAuthor};

use diesel;
use diesel::result::Error;
use diesel::pg::PgConnection;
use diesel::prelude::*;

pub fn load_or_create(conn: &PgConnection, author_name: &str, author_email: &str) -> Author {
    use schema::authors::dsl::*;
    use diesel::associations::HasTable;

    match authors
        .filter(name.eq(author_name))
        .filter(email.eq(author_email))
        .first(conn) {
            Ok(author) => author,
            Err(Error::NotFound) => {
                let new_author = NewAuthor {
                    name: author_name,
                    email: author_email,
                };
                diesel::insert(&new_author)
                    .into(authors::table())
                    .get_result(conn)
                    .expect("Error saving new author")
            },
            Err(_) => panic!("Error loading author from the datebase")
        }
}
