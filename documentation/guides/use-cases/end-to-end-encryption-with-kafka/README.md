# End-to-end encrypted messaging with Ockam and Kafka

https://cwiki.apache.org/confluence/display/KAFKA/KIP-317%3A+Add+end-to-end+data+encryption+functionality+to+Apache+Kafka

Running the example:


```
MODE=responder ./ockam_kafka_e2ee

Created kafka topics.

Receiving messages from: generated_abc
Sending messages to: generated_dfe

Use the following arguments to run the other node:

IN=generated_dfe OUT=generated_abc MODE=initiator ./ockam_kafka_e2ee

Waiting for secure channel...

Secure channel established

Received message <encrypted blah>

Secure channel decrypted message: blah

```


```
IN=generated_dfe OUT=generated_abc MODE=initiator ./ockam_kafka_e2ee

Initiated stream
Established secure channel

Secure channel established

Enter your message:

>>> blah

Message sent through secure channel

Secure channel encrypted message: <encrypted blah>


```