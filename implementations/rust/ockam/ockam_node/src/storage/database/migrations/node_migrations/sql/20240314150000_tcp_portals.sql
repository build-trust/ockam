CREATE TABLE tcp_inlet_status
(
    node_name    TEXT NOT NULL, -- Node where that tcp inlet has been created
    bind_addr    TEXT NOT NULL, -- Input address to connect to
    worker_addr  TEXT NOT NULL, -- Worker address for the inlet itself
    alias        TEXT NOT NULL, -- Alias for that inlet
    payload      TEXT,          -- Optional status payload
    outlet_route TEXT NULL,     -- Optional route to the outlet
    outlet_addr  TEXT NOT NULL  -- Address of the outlet worker
);
