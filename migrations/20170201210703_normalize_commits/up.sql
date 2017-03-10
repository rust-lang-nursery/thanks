ALTER TABLE commits ADD COLUMN author_id integer;

INSERT INTO authors (name, email)
	SELECT author_name, author_email FROM commits GROUP BY author_name, author_email;

UPDATE commits SET author_id = (
	SELECT id FROM authors WHERE name = author_name AND email = author_email
);

ALTER TABLE commits
	ALTER COLUMN author_id SET NOT NULL,
	ADD CONSTRAINT authors
	FOREIGN KEY (author_id)
	REFERENCES authors (id)
	ON DELETE CASCADE,
	DROP COLUMN author_name,
	DROP COLUMN author_email;

CREATE INDEX commits_author_id_idx ON commits USING btree (author_id);
