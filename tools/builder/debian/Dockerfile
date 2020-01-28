# Stage 1: Create the base image with all necessary components
FROM debian:9 as vagrant-base

# Configure the debconf frontend.
#
# This image doesn't include whiptail, dialog, nor the readline perl module.
# Therefore, the debconf frontend will be set to 'teletype' to avoid error
# messages about no dialog frontend could be found.
RUN echo 'debconf debconf/frontend select teletype' | debconf-set-selections

# Install the necessary packages.
#
# In addition to the regular Debian base image, a BASIC set of packages from the
# Debian minimal configuration will be installed. After all packages have been
# installed, the apt caches and some log files will be removed to minimize the
# image.
#
# NOTE: An upgrade will be performed to include updates and security fixes of
#       installed packages that received updates in the Debian repository after
#       the upstream image has been created.
#
# NOTE: No syslog daemon will be installed, as systemd's journald should fit
#       most needs. Please file an issue if you think this should be changed.
RUN apt-get update && \
    apt-get dist-upgrade -y && \
    apt-get install -y --no-install-recommends \
        apt-transport-https \
        ca-certificates \
        sudo \
        systemd      \
        systemd-sysv \
        cron         \
        anacron \
        curl \
        wget \
        build-essential \
        gnupg \
        openssh-server \
        openssh-client && \
    apt-get clean

RUN systemctl enable ssh.service

# Configure systemd.
#
# For running systemd inside a Docker container, some additional tweaks are
# required. For a detailed list see:
#
# https://developers.redhat.com/blog/2016/09/13/ \
#   running-systemd-in-a-non-privileged-container/
#
# Additional tweaks will be applied in the final image below.

# To avoid ugly warnings when running this image on a host running systemd, the
# following units will be masked.
#
# NOTE: This will not remove ALL warnings in all Debian releases, but seems to
#       work for stretch.
RUN systemctl mask --   \
    dev-hugepages.mount \
    sys-fs-fuse-connections.mount

# The machine-id should be generated when creating the container. This will be
# done automatically if the file is not present, so let's delete it.
RUN rm -f           \
    /etc/machine-id \
    /var/lib/dbus/machine-id

# Create vagrant user, add to sudoers, create /vagrant
RUN \
    useradd vagrant && \
    echo "vagrant:vagrant" | chpasswd && \
    usermod -a -G root vagrant && \
    echo 'vagrant ALL=(ALL) NOPASSWD:ALL' > /etc/sudoers.d/99_vagrant && \
    chmod 440 /etc/sudoers.d/99_vagrant && \
    mkdir -p /home/vagrant && \
    mkdir -p /vagrant && \
    chown -R vagrant:vagrant /home/vagrant /vagrant

# Set locale to en_US.UTF-8
RUN \
    apt-get -y install locales && \
    echo 'en_US.UTF-8 UTF-8' >> /etc/locale.gen && \
    locale-gen && \
    mkdir -p /etc/profile.d && \
    echo 'LANG=en_US.UTF-8' >> /etc/profile.d/locale.sh

# From now on we run as the vagrant user
USER vagrant:vagrant

# Use the vagrant HOME directory as our primary working directory
WORKDIR /home/vagrant

# Load in external files we need for provisioning
ADD files /tmp/files
ADD provision /tmp/provision

# Run all the provisioning scripts with sudo, in the same order as virtualbox
RUN \
    echo 'vagrant' | sudo -S -E cp /tmp/files/download /usr/bin/ && \
    sudo chmod +x /tmp/provision/*.sh && \
    sudo /tmp/provision/vagrant.sh && \
    sudo /tmp/provision/sshd.sh && \
    sudo /tmp/provision/common.sh && \
    sudo /tmp/provision/shellcheck.sh && \
    sudo /tmp/provision/crosstoolng.sh && \
    sudo /tmp/provision/cmake.sh && \
    sudo /tmp/provision/node.sh && \
    sudo /tmp/provision/go.sh && \
    sudo /tmp/provision/rust.sh && \
    sudo /tmp/provision/java.sh && \
    sudo /tmp/provision/android.sh && \
    sudo /tmp/provision/erlang.sh && \
    sudo /tmp/provision/elixir.sh && \
    sudo rm -rf /tmp/*

# Stage 2: Configure base image for use as a Vagrant builder
#
# The method here copies all of the files from the base image
# without the deleted files, resulting in a minimal image with
# a single layer
FROM debian:9 as vagrant-builder
COPY --from=0 / /

# We load up the user public key as an authorized key for direct
# SSH. Vagrant handles loading in its own key, so we don't need
# to do that
ARG public_key

# Configure systemd.
#
# For running systemd inside a Docker container, some additional tweaks are
# required. Some of them have already been applied above.
#
# The 'container' environment variable tells systemd that it's running inside a
# Docker container environment.
ENV container docker

# Install personal public key for SSH directly to container
# NOTE: We copy the authorized_keys file from the vagrant user, since
# that is where the provision scripts place it
# NOTE: We echo an extra line because the provision scripts above
# trim off newlines, so we need to add an extra one
RUN echo '' >> /home/vagrant/.ssh/authorized_keys && \
    echo "${public_key}" >> /home/vagrant/.ssh/authorized_keys

# A different stop signal is required, so systemd will initiate a shutdown when
# running 'docker stop <container>'.
STOPSIGNAL SIGRTMIN+3

# Need to expose port 22 for SSH
EXPOSE 22

# The image boots straight into init
CMD ["/bin/systemd", "--system"]
