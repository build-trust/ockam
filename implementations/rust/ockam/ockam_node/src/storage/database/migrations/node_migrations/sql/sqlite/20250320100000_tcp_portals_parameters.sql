-- create table tcp_inlet
-- (
--     node_name   TEXT not null,
--     bind_addr   TEXT not null,
--     outlet_addr TEXT not null,
--     alias       TEXT not null
-- );

ALTER TABLE tcp_outlet_status RENAME TO tcp_outlet;
ALTER TABLE tcp_outlet DROP COLUMN payload;
ALTER TABLE tcp_outlet RENAME COLUMN socket_addr TO "to";
ALTER TABLE tcp_outlet RENAME COLUMN worker_addr TO worker_address;

ALTER TABLE tcp_outlet ADD COLUMN policy_expression TEXT;
ALTER TABLE tcp_outlet ADD COLUMN tls INTEGER;
ALTER TABLE tcp_outlet ADD COLUMN skip_handshake INTEGER;
ALTER TABLE tcp_outlet ADD COLUMN enable_nagle INTEGER;
