docker build --rm --platform linux/amd64 -t $REPOSITORY_URL/ockam_kafka_inlet:ki ./kafka_ingest/application/services/ockam_kafka_inlet
snow spcs image-registry login
docker push $REPOSITORY_URL/ockam_kafka_inlet:ki

envsubst < ./kafka_ingest/application/services/spec.template.yml > ./kafka_ingest/application/services/spec.yml
snow app run --project ./kafka_ingest/application
