FROM node:lts-alpine3.17
RUN apk add --update --no-cache python3 g++ make bash openssl-dev curl

RUN curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/build-trust/ockam/develop/install.sh | bash
ENV PATH="/root/.ockam/bin:$PATH"
COPY ./start_ockam.sh /start_ockam.sh
COPY ./entrypoint.sh /entrypoint.sh

COPY ./* ./

RUN chmod +x /start_ockam.sh
RUN chmod +x /entrypoint.sh

RUN npm install
ENTRYPOINT [ "/entrypoint.sh"]