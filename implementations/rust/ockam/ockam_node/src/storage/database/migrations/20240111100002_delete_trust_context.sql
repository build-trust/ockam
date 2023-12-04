DROP TABLE trust_context;

CREATE INDEX identity_attributes_attested_by_index ON identity_attributes (identifier, attested_by);

-- Old policies contain trust context id, they should be recreated
DELETE FROM policy;
