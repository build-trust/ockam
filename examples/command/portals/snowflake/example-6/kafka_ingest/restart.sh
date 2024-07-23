
docker build --rm --platform linux/amd64 -t $REPOSITORY_URL/ockam_kafka_inlet:ki ./kafka_ingest/application/services/ockam_kafka_inlet
docker build --rm --platform linux/amd64 -t $REPOSITORY_URL/kafka_consumer:ki ./kafka_ingest/application/services/kafka_consumer
snow spcs image-registry login
docker push $REPOSITORY_URL/ockam_kafka_inlet:ki
docker push $REPOSITORY_URL/kafka_consumer:ki

mkdir ./kafka_ingest/target 2>&1 /dev/null
cp -r ./kafka_ingest/application ./kafka_ingest/target
envsubst < ./kafka_ingest/application/services/spec.yml > ./kafka_ingest/target/application/services/spec.yml

snow app run --project ./kafka_ingest/target/application

#snow sql --query "CALL kafka_ingest.functions.stop_application();" --role ki_role --warehouse ki_warehouse
#snow sql --query "CALL kafka_ingest.functions.start_application();" --role ki_role --warehouse ki_warehouse
