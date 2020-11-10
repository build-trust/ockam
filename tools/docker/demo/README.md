# Demo 2 setup instructions

1. build local docker images
```sh
# from `ockam` repo root
docker build -t ockam/ockam-dev -f tools/docker/ockam-dev/Dockerfile .
docker build -t ockam/ockamd:0.10.1 -f tools/docker/rust/Dockerfile.ockamd .
docker build -t ockam/influxdb-ockamd-via-hub:0.10.1 -f tools/docker/influxdb/Dockerfile.influxdb-ockamd .
docker build -t ockam/telegraf-ockamd-via-hub:0.10.1 -f tools/docker/telegraf/Dockerfile.telegraf-ockamd .
```

2. run the demo steps
```sh
./tools/docker/demo/influxdb.sh influxdb-ockamd-via-ockam-hub
./tools/docker/demo/influxdb.sh telegraf-ockamd-via-ockam-hub
./tools/docker/demo/influxdb.sh telegraf-write
./tools/docker/demo/influxdb.sh influxdb-query
```