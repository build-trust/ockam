-- Add support for remote ockam vaults
ALTER TABLE vault
    ADD COLUMN vault_multiaddr TEXT;
ALTER TABLE vault
    ADD COLUMN local_identifier TEXT;
ALTER TABLE vault
    ADD COLUMN authority_identifier TEXT;
ALTER TABLE vault
    ADD COLUMN authority_multiaddr TEXT;
ALTER TABLE vault
    ADD COLUMN credential_scope TEXT;
