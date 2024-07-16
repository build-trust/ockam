-- Remove 'alias' column from the tcp_outlet_status table
-- Since the 'alias' column was set as the primary key, we
-- need to recreate the table.
CREATE TABLE tcp_outlet_status_copy
(
    socket_addr TEXT NOT NULL,    -- Socket address that the outlet connects to
    worker_addr TEXT NOT NULL,    -- Worker address for the outlet itself
    payload     TEXT              -- Optional status payload
);
INSERT INTO tcp_outlet_status_copy (socket_addr, worker_addr, payload)
    SELECT socket_addr, worker_addr, payload FROM tcp_outlet_status;
DROP TABLE tcp_outlet_status;
ALTER TABLE tcp_outlet_status_copy RENAME TO tcp_outlet_status;
