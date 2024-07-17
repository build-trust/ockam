docker build --rm --platform linux/amd64 -t $REPOSITORY_URL/cdc_publisher:cdc ./cdc_publisher/application/services/cdc_publisher
snow spcs image-registry login
docker push $REPOSITORY_URL/cdc_publisher:cdc
snow app run --project ./cdc_publisher/application
snow sql --query "CALL cdc_publisher.functions.stop_application();" --role cdc_role --warehouse cdc_warehouse
snow sql --query "CALL cdc_publisher.functions.start_application();" --role cdc_role --warehouse cdc_warehouse
