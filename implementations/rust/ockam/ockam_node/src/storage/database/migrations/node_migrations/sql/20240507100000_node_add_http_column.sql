-- Add a column to the node table to store the optional http server address
ALTER TABLE node
    ADD COLUMN http_server_address TEXT;
