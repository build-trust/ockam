## Postgres client

This is a test application to verify that an Ockam node application is working. This application is
supposed to be deployed in the same account as the Ockam node application.

Deploy with

```shell
snow app run
```

Start the service inside a worksheet with:

```sqlite-sql
CALL external.start_postgres_client('ockam-endpoint', '5432', 'postgres')
```
