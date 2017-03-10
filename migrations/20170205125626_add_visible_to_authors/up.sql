ALTER TABLE authors ADD COLUMN visible BOOLEAN NOT NULL DEFAULT TRUE;

CREATE INDEX authors_visible_idx ON authors(visible) WHERE visible = TRUE;
