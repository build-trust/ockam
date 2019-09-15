#!/bin/sh -eux

version=$(cat /home/vagrant/.vbox_version)

mkdir -p /tmp/vbox
mount -o loop "/home/vagrant/VBoxGuestAdditions_$version.iso" /tmp/vbox

sh /tmp/vbox/VBoxLinuxAdditions.run || echo "VBoxLinuxAdditions.run exit code $? is suppressed"

umount /tmp/vbox
rm -rf /tmp/vbox
rm -f /home/vagrant/*.iso
