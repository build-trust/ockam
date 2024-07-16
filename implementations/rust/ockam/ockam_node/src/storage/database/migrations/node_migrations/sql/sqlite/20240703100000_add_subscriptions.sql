-- This migration adds the subscriptions table based on the following rust struct
CREATE TABLE subscription (
    space_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    is_free_trial INTEGER NOT NULL,
    marketplace TEXT,
    start_date INTEGER,
    end_date INTEGER
);
