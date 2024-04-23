# Instaclustr

In this hands-on example we send end-to-end encrypted messages _through_ Instaclustr.

[<mark style="color:blue;">Ockam</mark>](https://docs.ockam.io/) encrypts messages from a Producer to a specific Consumer. Only that specific Consumer can decrypt these messages. This guarantees that your data cannot be observed or tampered as it passes through Instaclustr. Operators of Instaclustr only see end-to-end encrypted data. Any compromise of an operator's infrastructure cannot compromise your business data.

The example uses docker and docker compose to create virtual networks.

You can read a detailed walkthrough of this example at:
https://docs.ockam.io/portals/kafka/instaclustr/docker
