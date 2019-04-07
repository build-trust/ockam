# Running

## build

`docker build -t resolver . --build-arg DISCOVERER_NAME="test.ockam.network" --build-arg DISCOVERER_PORT=26657`

## run
`docker run -p 8080:8080 resolver --rm`

