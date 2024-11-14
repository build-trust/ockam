FROM mcr.microsoft.com/vscode/devcontainers/base:0.202.7-bullseye

ARG NIX_VERSION=2.18.1
ARG CHANNEL_URL=https://github.com/NixOS/nixpkgs/archive/1db42b7fe3878f3f5f7a4f2dc210772fd080e205.tar.gz

ARG USER=vscode
ARG UID=1000
ARG GID=${UID}
ARG NIX_INSTALLER=https://releases.nixos.org/nix/nix-${NIX_VERSION}/install

SHELL [ "/bin/bash", "-o", "pipefail", "-o", "errexit", "-c" ]

RUN export DEBIAN_FRONTEND=noninteractive && \
    apt-get update && \
    apt-get install --yes --no-install-recommends \
    ca-certificates \
    curl \
    locales \
    make \
    acl \
    sudo \
    xz-utils

RUN usermod --append --groups sudo ${USER} --shell /bin/bash && \
    usermod --append --groups root ${USER} --shell /bin/bash && \
    adduser ${USER} root && \
    sed --in-place 's/%sudo.*ALL/%sudo   ALL=(ALL:ALL) NOPASSWD:ALL/' /etc/sudoers

RUN mkdir --parents /etc/nix/ && \
    echo "sandbox = relaxed" >> /etc/nix/nix.conf && \
    echo "experimental-features = nix-command flakes" >> /etc/nix/nix.conf && \
    passwd --delete root

USER ${USER}
ENV USER=${USER}

ENV LOCALE_ARCHIVE=/usr/lib/locale/locale-archive
ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US:en
ENV LC_ALL=en_US.UTF-8

# Generate en_US.UTF-8 locale
RUN locale-gen en_US.UTF-8 && \
    sudo dpkg-reconfigure --frontend=noninteractive locales && \
    sudo locale-gen en_US.UTF-8

RUN curl --location ${NIX_INSTALLER} > ~/install.sh && \
    chmod +x ~/install.sh  && \
    ~/install.sh

RUN source ~/.nix-profile/etc/profile.d/nix.sh && \
    nix-channel --add ${CHANNEL_URL} nixpkgs && \
    nix-channel --update

RUN echo "source ~/.nix-profile/etc/profile.d/nix.sh" >> ~/.profile && \
    echo "source ~/.nix-profile/etc/profile.d/nix.sh" >> ~/.bashrc

WORKDIR /home/${USER}/

COPY --chown=${USER}:${USER} . .

RUN source ~/.nix-profile/etc/profile.d/nix.sh && \
    nix-env --set-flag priority 10 nix-${NIX_VERSION} && \
    export "ARCH_OS=$(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]')"
