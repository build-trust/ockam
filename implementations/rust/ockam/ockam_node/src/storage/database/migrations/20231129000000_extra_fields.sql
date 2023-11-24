-- SQLite does not support ALTER TABLE DROP COLUMN, so we have to create a new table and copy the data over.
-- Create a new table `local_service` without the `payload` column and with the `scheme` column
CREATE TABLE local_service
(
  alias       TEXT PRIMARY KEY, -- Name for the local service
  socket_addr TEXT NOT NULL,    -- Socket address that the local service connects to
  worker_addr TEXT NOT NULL,    -- Worker address for the outlet
  scheme      TEXT              -- The URL scheme for the local service
);

-- Populate the new table with data from the existing `tcp_outlet_status` table
INSERT INTO local_service(alias, socket_addr, worker_addr)
SELECT alias, socket_addr, worker_addr FROM tcp_outlet_status;

-- Drop the old table
DROP TABLE tcp_outlet_status;

-- Optional port, to allow a fixed port for an incoming service
-- Should be unique since you cannot listen on the same port
-- for multiple services
ALTER TABLE incoming_service ADD COLUMN port INTEGER;

-- The URL scheme for the incoming service
ALTER TABLE incoming_service ADD COLUMN scheme TEXT;
