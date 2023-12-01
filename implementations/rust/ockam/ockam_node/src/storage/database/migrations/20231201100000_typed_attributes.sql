--------------
-- IDENTITIES
--------------

ALTER TABLE identity_attributes RENAME TO identity_attributes_old;

-- This new table contains attributes stored on one row for each attribute name/attribute value pair.
-- The data migration:
--   - migrates the data from identity_attributes to identity_typed_attributes
--   - drops identity_attributes
--   - renames identity_typed_attributes to identity_attributes
CREATE TABLE identity_attributes
(
    identifier      TEXT NOT NULL,     -- identity possessing those attributes
    attribute_name  TEXT NOT NULL,     -- attribute name
    attribute_value TEXT NOT NULL,     -- attribute value
    added           INTEGER NOT NULL,  -- UNIX timestamp in seconds: when those attributes were inserted in the database
    expires         INTEGER,           -- optional UNIX timestamp in seconds: when those attributes expire
    attested_by     TEXT               -- optional identifier which attested of these attributes
);

CREATE UNIQUE INDEX identity_attributes_index ON identity_attributes (identifier, attribute_name, added);
