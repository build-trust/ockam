# Suborbital and Ockam

<p><img alt="suborbital and ockam" src="./ockam-suborbital.png"></p>

This integration is a reference design for a secure-by-default edge computing network using [Suborbital's](https://suborbital.dev) WebAssembly-based server application environment and Ockam's end-to-end encrypted communication channels. By utilising the sandboxing properties of WebAssembly and the encryption capabilities provided by Ockam, anyone can deploy edge computing infrastructure that protects users' data in transit and prevent security incidents caused by malicious code or supply-chain attacks.

As you can see in the diagram above, this example involves a webserver ([Atmo-proxy](https://github.com/suborbital/atmo)) deployed on a cloud instance, and an edge compute server ([Sat](https://github.com/suborbital/sat)) deployed anywhere in the world (such as home computers, Raspberry Pis, etc). HTTP traffic is received by the webserver, shuttled over the Ockam secure channel to the Sat instance, and handled by the WebAssembly function it has loaded.

## 1. Preparation
To deploy this reference design you'll need Docker and the [Subo CLI](https://github.com/suborbital/subo).

If you use macOS, you can use [Homebrew](https://brew.sh) to install the `subo` command line tool:

```bash
brew tap suborbital/subo
brew install subo
```
To install on Linux (or macOS without Homebrew), visit the [Subo repository](https://github.com/suborbital/subo/releases).

Then, run `subo --version` to ensure the installation was successful.


## 2. Build the WebAssembly function
The source code for the `helloworld-rs` function is included in this directory. To build it, use `Subo`:
```bash
cd integrations/suborbital
subo build .
```
Subo will use a Docker-based builder to compile the Rust serverless function into WebAssembly and create all of the artifacts needed to run this application.


## 3. Start Atmo-proxy and Ockam Outlet
This component is ideally deployed on a cloud instance such as a DigitalOcean droplet or Google Cloud Virtual Machine, but can be done on your local machine.
```bash
docker-compose -f docker-compose-ockam-tcp-outlet-atmo.yaml up
```
This will print a FORWARDING_ADDRESS for this outlet on Ockam Hub. Copy it. Atmo-proxy will begin listening on port `8080`


## 4. Start Ockam Inlet and Sat
You can now start the Sat edge compute server on any machine, and the Ockam secure channel will automatically be created to link the components, even if they are in isolated private networks.
```
FORWARDING_ADDRESS=FWD_05ea353a2d7b8261 docker-compose -f docker-compose-ockam-tcp-inlet-sat.yaml up
```

Replace `FWD_05ea353a2d7b8261` here with address from step 1.

Send a request to the `Atmo-proxy` server:
```bash
curl -d "my friend" {atmo-proxy-address}:8080/hello
hello, my friend
```

You have now successfully deployed an end-to-end encrypted and sandboxed edge compute network!