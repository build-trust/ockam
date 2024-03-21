-- Add email that was used during the enrollment
ALTER TABLE identity_enrollment
    ADD email TEXT;
