#!/bin/bash

# Create the .ssh directory if it doesn't exist
mkdir -p /home/ec2-user/.ssh

# Create the config file
cat > /home/ec2-user/.ssh/config <<EOL
Host 127.0.0.1
    Port 1222
    StrictHostKeyChecking no
    IdentityFile ~/gitlab_rsa
EOL

# Set appropriate permissions for the .ssh directory and config file
chown -R ec2-user:ec2-user /home/ec2-user/.ssh
chmod 700 /home/ec2-user/.ssh
chmod 600 /home/ec2-user/.ssh/config
