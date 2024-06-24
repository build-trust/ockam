#!/bin/bash

# Update and install required packages
echo 'Installing required packages'
sudo yum update -y
sudo yum install -y docker
sudo yum install -y jq

# Start and enable Docker service
echo 'Starting and enabling Docker service'
sudo systemctl start docker
sudo systemctl enable docker

# Add ec2-user to docker group
echo 'Adding ec2-user to docker group'
sudo usermod -aG docker ec2-user

# Ensure permissions for Docker socket
echo 'Setting permissions for Docker socket'
sudo chmod 666 /var/run/docker.sock

# Create GitLab directory
mkdir -p /home/ec2-user/gitlab
chown ec2-user:ec2-user /home/ec2-user/gitlab
chmod 0755 /home/ec2-user/gitlab

# Get metadata token
TOKEN=$(curl -X PUT "http://169.254.169.254/latest/api/token" -H "X-aws-ec2-metadata-token-ttl-seconds: 21600" -s)

# Fetch the public IP address of the instance
PUBLIC_IP=$(curl -H "X-aws-ec2-metadata-token: $TOKEN" -s http://169.254.169.254/latest/meta-data/public-ipv4)

# Create Docker network
docker network create gitlab-network

# Run GitLab container using Docker Engine
docker run --detach \
  --hostname "$PUBLIC_IP" \
  --env GITLAB_OMNIBUS_CONFIG="external_url 'http://$PUBLIC_IP'" \
  --publish 80:80 --publish 222:22 \
  --name gitlab \
  --restart always \
  --volume /srv/gitlab/config:/etc/gitlab \
  --volume /srv/gitlab/logs:/var/log/gitlab \
  --volume /srv/gitlab/data:/var/opt/gitlab \
  --network gitlab-network \
  gitlab/gitlab-ce:latest

# Wait for GitLab to become available
echo "Waiting for GitLab to become available..."
MAX_ATTEMPTS=10
WAIT_SECONDS=60
ATTEMPT_NUM=1

while : ; do
    echo "Checking GitLab status, attempt $ATTEMPT_NUM..."
    STATUS_CODE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost)
    if [ "$STATUS_CODE" = "200" ] || [ "$STATUS_CODE" = "302" ]; then
        echo "GitLab is up and running. Status code: $STATUS_CODE"
        break
    else
        if [ $ATTEMPT_NUM -eq $MAX_ATTEMPTS ]; then
            echo "Reached maximum number of attempts. GitLab might still be starting up or there could be an issue with the configuration. Last status code: $STATUS_CODE"
            break
        else
            echo "GitLab not yet available, waiting $WAIT_SECONDS seconds..."
            sleep $WAIT_SECONDS
            ((ATTEMPT_NUM++))
        fi
    fi
done

USERNAME=root
PASSWORD=$(sudo docker exec gitlab grep 'Password:' /etc/gitlab/initial_root_password | cut -d ' ' -f 2)

# Fetch the access token
ACCESS_TOKEN=$(curl -X POST -H "Content-Type: application/json" -d "{\"grant_type\":\"password\",\"username\":\"$USERNAME\",\"password\":\"$PASSWORD\"}" "http://localhost/oauth/token" | jq -r '.access_token')

# Disable sign-ups using the GitLab API
echo "Disabling sign-ups using the GitLab API..."
curl -X PUT -H "Content-Type: application/json" -H "Authorization: Bearer $ACCESS_TOKEN" -d "{\"signup_enabled\":false}" "http://localhost/api/v4/application/settings"
echo "Sign-ups disabled successfully."

# Create the project
echo "Creating project '$PROJECT_NAME' under the root user using the GitLab API..."
PROJECT_NAME="demo_project"
PROJECT_DESCRIPTION="Demo project for testing"

curl -X POST -H "Content-Type: application/json" -H "Authorization: Bearer $ACCESS_TOKEN" -d "{\"name\":\"$PROJECT_NAME\",\"description\":\"$PROJECT_DESCRIPTION\",\"visibility\":\"private\",\"initialize_with_readme\":true}" "http://localhost/api/v4/projects"
echo "Project '$PROJECT_NAME' created successfully."

# Public Key value updated by run.sh
PUBLIC_KEY=$SSH_PUBLIC_KEY

# Add the SSH key to the root user using the GitLab API
echo "Adding SSH key to the root user..."
curl -X POST -H "Content-Type: application/json" -H "Authorization: Bearer $ACCESS_TOKEN" -d "{\"title\":\"GitLab SSH Key\",\"key\":\"$PUBLIC_KEY\"}" "http://localhost/api/v4/user/keys"

echo "SSH key added successfully."
echo "Setup complete"
