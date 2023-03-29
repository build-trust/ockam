FROM influxdb@sha256:1db410a831ae62beaf59f2d8f6176a37cdd54cd3ffa12d6afc4e86fac8c4aa12
RUN curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/build-trust/ockam/ockam_v0.82.0/install.sh | sh
RUN mv ockam /usr/local/bin
RUN chmod +x /usr/local/bin/ockam

COPY ./start_ockam.sh /start_ockam.sh
RUN chmod +x /start_ockam.sh
COPY ./entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
