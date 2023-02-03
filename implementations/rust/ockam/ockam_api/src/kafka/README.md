# Ockam Kafka Sidecar

This sidecar allows encrypted transparent communication from the kafka client to the kafka cluster without any
modification in the existing application.

```
ockam service start kafka-producer --ip 127.0.0.1 --forwarding-addr /dnsaddr/<project-hostname>/tcp/<project-port>/service/service/kafka_interceptor --bootstrap-port 4444 --port-range 20000-40000
```
