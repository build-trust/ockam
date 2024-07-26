docker build --rm --platform linux/amd64 -t $REPOSITORY_URL/ockam_node:on ./ockam_node/application/ockam_node
docker push $REPOSITORY_URL/ockam_node:on

docker build --rm --platform linux/amd64 -t $REPOSITORY_URL/postgres_client:on ./ockam_node/application/postgres_client
docker push $REPOSITORY_URL/postgres_client:on

snow app run --project ./ockam_node/application --role on_role
