FROM atmoz/sftp:latest

COPY sftp_host_rsa_key.pub /home/foo/.ssh/keys/sftp_host_rsa_key.pub
RUN chmod 444 /home/foo/.ssh/keys/sftp_host_rsa_key.pub
