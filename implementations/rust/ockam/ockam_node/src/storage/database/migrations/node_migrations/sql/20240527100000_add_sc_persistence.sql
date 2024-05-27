CREATE TABLE secure_channel
(
    role                        INTEGER NOT NULL,
    my_identifier               TEXT NOT NULL,
    their_identifier            TEXT NOT NULL,
    decryptor_remote_address    TEXT NOT NULL,
    decryptor_api_address       TEXT NOT NULL,
    decryption_key              TEXT NOT NULL,
    -- TODO: Add date?
    node_name                   TEXT NOT NULL     -- node name to isolate credential that each node has
);

CREATE UNIQUE INDEX secure_channel_decryptor_api_address_index ON secure_channel(decryptor_remote_address, node_name);
