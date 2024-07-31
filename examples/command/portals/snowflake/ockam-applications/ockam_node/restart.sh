docker build --rm --platform linux/amd64 -t $REPOSITORY_URL/ockam_node:on ./application/ockam_node

snow spcs image-registry login
docker push $REPOSITORY_URL/ockam_node:on

snow app run --project ./application --role on_role
