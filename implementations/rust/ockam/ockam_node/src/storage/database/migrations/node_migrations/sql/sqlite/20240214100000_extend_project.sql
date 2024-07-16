ALTER TABLE project
    RENAME COLUMN identifier TO project_identifier;
ALTER TABLE project
    RENAME COLUMN authority_identity TO authority_change_history;
ALTER TABLE project
    ADD COLUMN project_change_history TEXT;
