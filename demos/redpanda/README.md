
Setup

```
ockam enroll

for n in outlet consumer producer; do
    mkdir -p mount/$n/secrets
    ockam project ticket > mount/$n/secrets/ticket
done

docker compose up -d
```

In there separate windows run:

```
docker-compose logs -f -t kafka-consumer-a
docker-compose logs -f -t kafka-consumer-b
docker exec -it kafka-producer kafka-console-producer.sh --topic demo-topic --bootstrap-server localhost:9092
```

Type new messages into the producer.
Consumer A will see unencryped messages.
Consumer B will only see encrypted data.
