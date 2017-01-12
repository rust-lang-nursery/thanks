CREATE TABLE releases (
	id SERIAL PRIMARY KEY,
	version VARCHAR NOT NULL
);

ALTER TABLE commits
ADD release_id integer NOT NULL
