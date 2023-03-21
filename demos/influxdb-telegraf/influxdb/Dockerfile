FROM influxdb:2.6
RUN curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/build-trust/ockam/ockam_v0.82.0/install.sh | sh
RUN mv ockam /usr/local/bin
RUN chmod +x /usr/local/bin/ockam

COPY ./start_ockam.sh /start_ockam.sh
RUN chmod +x /start_ockam.sh
COPY ./entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
