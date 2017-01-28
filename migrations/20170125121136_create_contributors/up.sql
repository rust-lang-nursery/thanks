CREATE TABLE contributors (
    id SERIAL NOT NULL,
    name VARCHAR NOT NULL,
    email VARCHAR NOT NULL
);

ALTER TABLE ONLY contributors
    ADD CONSTRAINT contributors_pkey PRIMARY KEY (id);

CREATE UNIQUE INDEX contributors_id_idx ON contributors USING btree (id);
