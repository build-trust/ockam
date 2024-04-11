FROM docker.redpanda.com/redpandadata/redpanda:v23.1.13

WORKDIR /var/lib/redpanda

# Install Ockam
RUN curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash -s
ENV PATH="/var/lib/redpanda/.ockam/bin:$PATH"

# Copy the script that will be used as entrypoint
COPY --chown=redpanda:redpanda run_ockam.sh /var/lib/redpanda/run_ockam.sh
RUN chmod +x /var/lib/redpanda/run_ockam.sh
ENTRYPOINT ["/var/lib/redpanda/run_ockam.sh"]
