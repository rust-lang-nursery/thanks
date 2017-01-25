ALTER TABLE ONLY releases
	ADD COLUMN project_id integer NOT NULL;

ALTER TABLE ONLY releases
	ADD CONSTRAINT projects
	FOREIGN KEY (project_id)
	REFERENCES projects (id)
	ON DELETE CASCADE;
