![Ockam Logo](https://www.ockam.io/0dc9e19beab4d96b8350d09be78361df/logo_white_background_preview.svg)

<p>
<a href="https://dev.azure.com/ockam-network/ockam/_build/latest?definitionId=10?branchName=develop">
<img alt="Build Status"
  src="https://dev.azure.com/ockam-network/ockam/_apis/build/status/ockam-network.ockam?branchName=develop">
</a>

<a href="https://www.ockam.io/learn/guides/team/conduct/">
<img alt="Contributor Covenant"
  src="https://img.shields.io/badge/Contributor%20Covenant-v2.0%20adopted-ff69b4.svg">
</a>

<a href="LICENSE">
<img alt="Apache 2.0 License"
  src="https://img.shields.io/badge/License-Apache%202.0-blue.svg?style=flat-square">
</a>
</p>

# `ockamd` 


```
ockamd 0.1.0
Ockam Developers (ockam.io)
Encrypt, route, and decrypt messages using the Ockam daemon.

USAGE:
    ockamd [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --addon <addon>                        Pre-defined configuration for an official Ockam Add-on, e.g.
                                               "influxdb,database_name,http://localhost:8086"
        --identity-name <identity-name>        Name of the private key to use for the identity of the channel initiator
                                               [default: 1.key]
        --input <input>                        Data source providing input to `ockamd` [default: stdin]
        --local-socket <local-socket>          Local node address and port to bind [default: 127.0.0.1:0]
        --public-key-hub <public-key-hub>      The public key provided by the hub service
        --public-key-sink <public-key-sink>    The public key provided by the remote (sink) service
        --role <role>                          Start `ockamd` as "source", "sink", or "router" of a secure channel
                                               [default: source]
        --route-hub <route-hub>                Hub address and port to establish a listening channel
        --route-sink <route-sink>              Route to responder (sink), e.g. udp://host:port[,udp://host:port] (note
                                               comma-separation) or "stdout" [default: stdout]
        --service-address <service-address>    Address used to reach the service on remote machine
        --vault <vault>                        Specify which type of Ockam vault to use for this instance of `ockamd`
                                               [default: FILESYSTEM]
        --vault-path <vault-path>              Filepath on disk to pre-existing private keys to be used by the
                                               filesystem vault [default: ockamd_vault]
```


**The Ockam Team is here to help you.**

If you still have questions after reading through our
[published content](https://www.ockam.io/learn), please reach out to us. We’d
love to connect with you to hear about what you are building.

## License and attributions

This code is licensed under the terms of the [Apache License 2.0](LICENSE)

This code depends on other open source packages; attributions for those
packages are in the [NOTICE](NOTICE) file.
