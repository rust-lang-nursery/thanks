use models::{Author, NewAuthor};

use diesel::*;
use diesel::pg::PgConnection;

pub fn load_or_create(conn: &PgConnection, author_name: &str, author_email: &str) -> Author {
    let new_author = NewAuthor {
        name: author_name,
        email: author_email,
    };

    find_or_create(conn, new_author)
        .expect("Could not find or create author")
}

pub fn find_or_create_all(conn: &PgConnection, new_authors: Vec<NewAuthor>)
    -> QueryResult<Vec<Author>>
{
    use schema::authors::dsl::*;
    use diesel::expression::dsl::any;
    use diesel::pg::upsert::*;

    let (names, emails): (Vec<_>, Vec<_>) = new_authors.iter()
        .map(|author| (author.name, author.email))
        .unzip();

    insert(&new_authors.on_conflict_do_nothing())
        .into(authors)
        .execute(conn)?;

    authors.filter(name.eq(any(names)))
        .filter(email.eq(any(emails)))
        .load(conn)
}

fn find_or_create(conn: &PgConnection, new_author: NewAuthor) -> QueryResult<Author> {
    use schema::authors::dsl::*;
    use diesel::pg::upsert::*;

    let maybe_inserted = insert(&new_author.on_conflict_do_nothing())
        .into(authors)
        .get_result(conn)
        .optional()?;

    if let Some(author) = maybe_inserted {
        return Ok(author);
    }

    authors.filter(name.eq(new_author.name))
        .filter(email.eq(new_author.email))
        .first(conn)
}
