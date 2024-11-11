-- Add privileged column to the tcp_outlet_status table
ALTER TABLE tcp_outlet_status
        ADD privileged BOOLEAN DEFAULT FALSE; -- boolean indicating if the outlet is operating in privileged mode

-- Add privileged column to the tcp_inlet table
ALTER TABLE tcp_inlet
        ADD privileged BOOLEAN DEFAULT FALSE; -- boolean indicating if the inlet is operating in privileged mode
