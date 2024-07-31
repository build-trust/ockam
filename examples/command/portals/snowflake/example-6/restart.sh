docker build --rm --platform linux/amd64 -t $REPOSITORY_URL/postgres_client ./application/postgres_client
docker push $REPOSITORY_URL/postgres_client

snow app run --project ./application --role consumer_role
