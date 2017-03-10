ALTER TABLE releases ADD COLUMN visible BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX releases_visible_idx ON releases(visible) WHERE visible = TRUE;
