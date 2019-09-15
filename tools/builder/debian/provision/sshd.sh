#!/bin/sh -eux

echo 'UseDNS no' >> /etc/ssh/sshd_config
echo 'PasswordAuthentication no' >> /etc/ssh/sshd_config
echo 'GSSAPIAuthentication no' >> /etc/ssh/sshd_config
echo '' >> /etc/ssh/sshd_config
