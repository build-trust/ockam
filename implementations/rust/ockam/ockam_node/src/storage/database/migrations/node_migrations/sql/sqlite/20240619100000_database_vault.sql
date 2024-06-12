-- This migration allows the path column to be NULL
-- When the path is NULL then the vault 'name' has all its keys stored in the current database
CREATE TABLE new_vault
(
    name       TEXT PRIMARY KEY, -- User-specified name for a vault
    path       TEXT NULL,        -- Path where the vault is saved
    is_default INTEGER,          -- boolean indicating if this vault is the default one (1 means true)
    is_kms     INTEGER           -- boolean indicating if this signing keys are stored in an AWS KMS (1 means true)
);

INSERT INTO new_vault (name, path, is_default, is_kms)
SELECT
  name,
  -- set the path to NULL when the vault is stored in the database
  CASE
    WHEN path LIKE '%database.sqlite3%' THEN NULL
    ELSE path
  END as path,
  -- fix the setting of the is_default flag which could occur more than once
  CASE
    WHEN name = 'default' THEN 1
    ELSE 0
  END as is_default,
  is_kms
FROM vault;

DROP TABLE vault;

ALTER TABLE new_vault RENAME TO vault;
