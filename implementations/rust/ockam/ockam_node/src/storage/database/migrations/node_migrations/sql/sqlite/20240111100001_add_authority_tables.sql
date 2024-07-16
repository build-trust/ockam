CREATE TABLE authority_member
(
    identifier     TEXT NOT NULL UNIQUE,
    added_by       TEXT NOT NULL,
    added_at       INTEGER NOT NULL,
    is_pre_trusted INTEGER NOT NULL,
    attributes     BLOB
);

CREATE UNIQUE INDEX authority_member_identifier_index ON authority_member(identifier);
CREATE INDEX authority_member_is_pre_trusted_index ON authority_member(is_pre_trusted);

CREATE TABLE authority_enrollment_token
(
    one_time_code TEXT NOT NULL UNIQUE,
    issued_by     TEXT NOT NULL,
    created_at    INTEGER NOT NULL,
    expires_at    INTEGER NOT NULL,
    ttl_count     INTEGER NOT NULL,
    attributes    BLOB
);

CREATE UNIQUE INDEX authority_enrollment_token_one_time_code_index ON authority_enrollment_token(one_time_code);
CREATE INDEX authority_enrollment_token_expires_at_index ON authority_enrollment_token(expires_at);
