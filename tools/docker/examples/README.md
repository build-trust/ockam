# Dockerized Example Runner

This Dockerfile creates an environment to build and run all of the Ockam examples.


# Building

The `./build.sh` shell script creates a local `ockam-example-runner:latest` image.

# Running

```shell
docker run -e OCKAM_HUB=127.0.0.1:4000 ockam-example-runner:latest
```
WIP
