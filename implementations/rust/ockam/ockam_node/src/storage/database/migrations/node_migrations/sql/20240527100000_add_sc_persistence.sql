CREATE TABLE secure_channel
(
    role                        TEXT NOT NULL,
    my_identifier               TEXT NOT NULL,
    their_identifier            TEXT NOT NULL,
    decryptor_remote_address    TEXT PRIMARY KEY,
    decryptor_api_address       TEXT NOT NULL,
    decryption_key_handle       BLOB NOT NULL
    -- TODO: Add date?
);

CREATE UNIQUE INDEX secure_channel_decryptor_api_address_index ON secure_channel(decryptor_remote_address);

-- This table stores aead secrets
CREATE TABLE aead_secret
(
    handle      BLOB PRIMARY KEY, -- Secret handle
    type        TEXT NOT NULL,    -- Secret type
    secret      BLOB NOT NULL     -- Secret binary
);
