ALTER TABLE identity_attributes RENAME TO identity_attributes_old;

-- This table lists attributes associated to a given identity
-- The data migration:
--   - duplicates every attributes entry for every node that exists at the time of migration
CREATE TABLE identity_attributes
(
    identifier  TEXT    NOT NULL, -- identity possessing those attributes
    attributes  BLOB    NOT NULL, -- serialized list of attribute names and values for the identity
    added       INTEGER NOT NULL, -- UNIX timestamp in seconds: when those attributes were inserted in the database
    expires     INTEGER,          -- optional UNIX timestamp in seconds: when those attributes expire
    attested_by TEXT,             -- optional identifier which attested of these attributes
    node_name   TEXT NOT NULL     -- node name to isolate attributes that each node knows
);

CREATE UNIQUE INDEX identity_attributes_index ON identity_attributes (identifier, node_name);
