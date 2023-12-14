FROM docker.redpanda.com/redpandadata/redpanda:v23.1.13

RUN curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/build-trust/ockam/develop/install.sh | bash
ENV PATH="/var/lib/redpanda/.ockam/bin:$PATH"
COPY --chown=redpanda:redpanda ./ockam.sh /var/lib/redpanda/ockam.sh
RUN chmod +x /var/lib/redpanda/ockam.sh

ENTRYPOINT ["/var/lib/redpanda/ockam.sh"]
CMD ["redpanda", "start", "--kafka-addr", "internal://0.0.0.0:9092,external://0.0.0.0:19092", \
    "--advertise-kafka-addr internal://redpanda-0:9092", \
    "--pandaproxy-addr internal://0.0.0.0:8082", \
    "--advertise-pandaproxy-addr internal://redpanda-0:8082", \
    "--schema-registry-addr internal://0.0.0.0:8081", \
    "--smp 1", \
    "--memory 1G", \
    "--mode dev-container", \
    "--default-log-level=debug"]
