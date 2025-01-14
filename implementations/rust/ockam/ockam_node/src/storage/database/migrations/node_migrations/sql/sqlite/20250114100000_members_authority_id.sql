-- Add a column to track the authority id
ALTER TABLE authority_member
    ADD authority_id TEXT NOT NULL DEFAULT 'fixme';
