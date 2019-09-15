#!/bin/sh -eux

# Disable systemd apt timers/services
systemctl stop apt-daily.timer
systemctl stop apt-daily-upgrade.timer
systemctl disable apt-daily.timer
systemctl disable apt-daily-upgrade.timer
systemctl mask apt-daily.service
systemctl mask apt-daily-upgrade.service
systemctl daemon-reload

# Disable periodic activities of apt
cat <<EOF >/etc/apt/apt.conf.d/10periodic
APT::Periodic::Enable "0";
APT::Periodic::Update-Package-Lists "0";
APT::Periodic::Download-Upgradeable-Packages "0";
APT::Periodic::AutocleanInterval "0";
APT::Periodic::Unattended-Upgrade "0";
EOF

apt-get update;

apt-get -y upgrade linux-image-amd64;
apt-get -y install linux-headers-"$(uname -r)";
