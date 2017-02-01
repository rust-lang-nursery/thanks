ALTER TABLE commits
	ADD COLUMN author_name VARCHAR,
	ADD COLUMN author_email VARCHAR;

UPDATE commits
	SET author_name = authors.name, author_email = authors.email
	FROM authors
	WHERE commits.author_id = authors.id;

ALTER TABLE commits DROP COLUMN author_id;

ALTER TABLE commits
	ALTER COLUMN author_name SET NOT NULL,
	ALTER COLUMN author_email SET NOT NULL;

TRUNCATE authors;
