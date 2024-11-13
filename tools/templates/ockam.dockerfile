FROM debian:stable-slim@sha256:8b49bae84e068b852725770ea01a0a08e461620da8006b69f8cc09c93d16d221 as builder

COPY assets .

RUN \
    set -xe; \
    ls; \
    case "$(uname -m)" in \
        aarch64*) \
            echo "ockam.aarch64-unknown-linux-gnu_sha256_value  ockam.aarch64-unknown-linux-gnu" | sha256sum -c; \
            mv ockam.aarch64-unknown-linux-gnu /ockam; \
            ;; \
        x86_64*) \
            echo "ockam.x86_64-unknown-linux-gnu_sha256_value  ockam.x86_64-unknown-linux-gnu" | sha256sum -c; \
            mv ockam.x86_64-unknown-linux-gnu /ockam; \
            ;; \
        *) \
            echo "unknown arch: $(uname -m)" \
            uname -a; \
            exit 1; \
        ;; \
    esac; \
    chmod u+x /ockam;

FROM cgr.dev/chainguard/glibc-dynamic:latest
COPY --chown=nonroot --from=builder /ockam /
ENTRYPOINT ["/ockam"]
