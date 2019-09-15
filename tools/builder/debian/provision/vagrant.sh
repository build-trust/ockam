#!/bin/sh -eux

# Set up password-less sudo for the vagrant user
echo 'vagrant ALL=(ALL) NOPASSWD:ALL' >/etc/sudoers.d/99_vagrant
chmod 440 /etc/sudoers.d/99_vagrant


cat <<EOF >/home/vagrant/.bashrc
export PATH=/usr/local/bin:/usr/bin:/bin

if [ -d /etc/profile.d ]; then
  for i in /etc/profile.d/*.sh; do
    if [ -r \$i ]; then
      . \$i
    fi
  done
  unset i
fi
EOF

mkdir -p /home/vagrant/.ssh
chown -R vagrant /home/vagrant
chmod -R go-rwsx /home/vagrant/.ssh

# Add vagrant insecure public key
# https://raw.githubusercontent.com/mitchellh/vagrant/master/keys/vagrant.pub
tr -d '\n' <<EOF > /home/vagrant/.ssh/authorized_keys
ssh-rsa AAAAB3NzaC1yc2EAAAABIwAAAQEA6NF8iallvQVp22WDkTkyrtvp9eWW6A8YVr+kz4TjGYe7gHzIw+niNlt
GEFHzD8+v1I2YJ6oXevct1YeS0o9HZyN1Q9qgCgzUFtdOKLv6IedplqoPkcmF0aYet2PkEDo3MlTBckFXPITAMzF8dJ
SIFo9D8HfdOV0IAdx4O7PtixWKn5y2hMNG0zQPyUecp4pzC6kivAIhyfHilFR61RGL+GPXQ2MWZWFYbAGjyiYJnAmCP
3NOTd0jMZEnDkbUvxhMmBYSdETk1rRgm+R4LOzFUGaHqHDLKLX+FIPKcF96hrucXzcWyLbIbEgE98OHlnVYCzRdK8jl
qm8tehUc9c9WhQ== vagrant insecure public key
EOF
