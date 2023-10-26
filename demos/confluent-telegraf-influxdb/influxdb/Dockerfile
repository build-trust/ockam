FROM influxdb:2.7

RUN curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/build-trust/ockam/develop/install.sh | bash
RUN mv /root/.ockam /home/influxdb
RUN chown -R influxdb:influxdb /home/influxdb/.ockam
ENV PATH="/home/influxdb/.ockam/bin:$PATH"

COPY ./start_ockam.sh /start_ockam.sh
RUN chmod +x /start_ockam.sh
COPY ./entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
