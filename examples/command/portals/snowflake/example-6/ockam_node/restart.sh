docker build --rm --platform linux/amd64 -t $REPOSITORY_URL/ockam_node:on ./ockam_node/application/ockam_node
snow spcs image-registry login
docker push $REPOSITORY_URL/ockam_node:on

envsubst < ./ockam_node/application/spec.template.yml > ./kafka_ingest/application/spec.yml
snow app run --project ./ockam_node/application

snow sql --query "CALL ockam_node.api.start_application();" --role on_role --warehouse on_warehouse
