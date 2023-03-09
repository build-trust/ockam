# Ockam + InfluxDB Demo

This is a demo that shows how to use Ockam to securely connect Telegraf and InfluxDB. A full run down on what is happening here and how it works can be read on the Ockam blog: [Connect distributed clients to a private InfluxDB database](https://www.ockam.io/blog/connect_private_influxdb)

## Prerequisites

* Ockam (`brew install build-trust/ockam/ockam`)
* Docker (including docker-compose)

## Setup

There's a script to setup the inital project (`influxdb-demo`) that our nodes
will later register with:

```bash
$ ./bin/setup
```

To start the InfluxDB server, Telegraf, and connect them via Ockam run:

```bash
$ ./bin/up
```

Telegraf will now be emitting and flushing system metrics to InfluxDB every 10 seconds. You
can verify that InfluxDB is receiving the data by using the `test` script to return the last
1 minute worth of data:

```bash
$ ./bin/test
```
