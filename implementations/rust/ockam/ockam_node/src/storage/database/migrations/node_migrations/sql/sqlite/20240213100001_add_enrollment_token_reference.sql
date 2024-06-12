-- Reference is a random string that uniquely identifies an enrollment token. However, unlike the one_time_code,
-- it's not sensitive so can be logged and used to track a lifecycle of a specific enrollment token.
ALTER TABLE authority_enrollment_token ADD COLUMN reference TEXT;
