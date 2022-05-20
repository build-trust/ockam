# Stage 1 - Download neccesary packages
FROM debian:11.1-slim@sha256:312218c8dae688bae4e9d12926704fa9af6f7307a6edb4f66e479702a9af5a0c as debian-packages

# Download all debian packages into this image so we can later copy them into the final runtime image.
WORKDIR /tmp
RUN set -xe; \
    apt-get update; \
    apt-get download \
        libc-bin; \
    mkdir -p /dpkg/var/lib/dpkg/status.d/ && \
    for deb in *.deb; do \
        package_name=$(dpkg-deb -I ${deb} | awk '/^ Package: .*$/ {print $2}'); \
        echo "Processing: ${package_name}"; \
        dpkg --ctrl-tarfile $deb | tar -Oxf - ./control > /dpkg/var/lib/dpkg/status.d/${package_name}; \
        dpkg --extract $deb /dpkg || exit 10; \
    done

# Remove unnecessary files extracted from deb packages like man pages and docs etc.
RUN find /dpkg/ -type d -empty -delete && \
    rm -r /dpkg/usr/share/doc/


# Stage 2 - Build elixir release of ockam_cloud_node elixir app
FROM ghcr.io/build-trust/ockam-builder@sha256:3ea4c222da220c69b62a60d42d83769286e07bf8e41ef7cfdbeba18b7f22a488 as elixir-app-release-build
COPY . /work
RUN set -xe; \
    cd implementations/elixir; \
    ../../gradlew build; \
    cd ockam/ockam_cloud_node; \
    MIX_ENV=prod mix release;


# Stage 3 - Create distroless container and copy executables and packages in above steps
FROM gcr.io/distroless/cc@sha256:3ca297cd5426268b5ad21e3fbe5c568411e0dec49dbae8e2967d33207bc99773

COPY --from=elixir-app-release-build /work/implementations/elixir/ockam/ockam_cloud_node/_build/prod/rel/ockam_cloud_node /opt/ockam_cloud_node

# Copy the libraries from the extractor stage into root
COPY --from=debian-packages /dpkg /

# We copy all shared libraries needed for elixir VM to function.
COPY --from=debian-packages /lib/x86_64-linux-gnu /lib
COPY --from=debian-packages /usr/lib/x86_64-linux-gnu/libpcre2-8.so.0 /lib
COPY --from=debian-packages /usr/lib/x86_64-linux-gnu/libacl.so.1 /lib
# We copy all programs needed for elixir VM to function.
COPY --from=debian-packages /bin/sh /bin/sh
COPY --from=debian-packages /bin/rm /bin/rm
COPY --from=debian-packages /usr/bin/cut /bin/cut
COPY --from=debian-packages /bin/readlink /bin/readlink
COPY --from=debian-packages /bin/sed /bin/sed
COPY --from=debian-packages /bin/cat /bin/cat
COPY --from=debian-packages /bin/grep /bin/grep
COPY --from=debian-packages /usr/bin/dirname /bin/dirname
COPY --from=debian-packages /usr/bin/basename /bin/basename

ENV LANG=C.UTF-8

EXPOSE 4000

ENTRYPOINT ["/opt/ockam_cloud_node/bin/ockam_cloud_node"]
CMD ["start"]
