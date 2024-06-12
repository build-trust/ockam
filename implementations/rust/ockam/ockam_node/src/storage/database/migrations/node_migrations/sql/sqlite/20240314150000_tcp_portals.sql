-- This migration create tables to store the active TCP inlets and outlets per node
CREATE TABLE tcp_inlet
(
    node_name    TEXT NOT NULL, -- Node where that tcp inlet has been created
    bind_addr    TEXT NOT NULL, -- Input address to connect to
    outlet_addr  TEXT NOT NULL, -- MultiAddress to the outlet
    alias        TEXT NOT NULL  -- Alias for that inlet
);

-- Add a node_name column to the tcp_outlet_status table
-- That table was previously only used to store the outlets created for the portal app
-- with node_name = 'ockam_app'
CREATE TABLE tcp_outlet_status_copy
(
    node_name   TEXT NOT NULL, -- Node where that tcp outlet has been created
    socket_addr TEXT NOT NULL, -- Socket address that the outlet connects to
    worker_addr TEXT NOT NULL, -- Worker address for the outlet itself
    payload     TEXT           -- Optional status payload
);
INSERT INTO tcp_outlet_status_copy (node_name, socket_addr, worker_addr, payload)
SELECT 'ockam_app', socket_addr, worker_addr, payload
FROM tcp_outlet_status;
DROP TABLE tcp_outlet_status;
ALTER TABLE tcp_outlet_status_copy
    RENAME TO tcp_outlet_status;
