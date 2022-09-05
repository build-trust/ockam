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
        armv7l*) \
            echo "ockam.armv7-unknown-linux-gnueabihf_sha256_value  ockam.armv7-unknown-linux-gnueabihf" | sha256sum -c; \
            mv ockam.armv7-unknown-linux-gnueabihf /ockam; \
            ;; \
        *) \
            echo "unknown arch: $(uname -m)" \
            uname -a; \
            exit 1; \
        ;; \
    esac; \
    chmod u+x /ockam;

FROM gcr.io/distroless/cc@sha256:1dc7ae628f0308f77dac8538b4b246453ac3636aa5ba22084e3d22d59a7f3cca
COPY --from=builder /ockam /
ENTRYPOINT ["/ockam"]
