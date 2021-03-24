# Ockam Credential Example

## Setup

```bash
cargo build
```


### Running the Example

**Step 1**: Start the issuer:
```shell
target/debug/issuer
```

You will see output similar to:
```shell
ockam_node::node: Initializing ockam node
Issuer listening on 0:issuer.
```

**Step 2**: Start the verifier:
```shell
target/debug/verifier
```

You will see output similar to:
```shell
ockam_node::node: Initializing ockam node
Verifier starting. Discovering Issuer
Discovered Issuer Pubkey: <hex string of the public key>
```

**Step 3**: Start the holder:
```shell
target/debug/holder
```

You will see output similar to:
```shell
ockam_node::node: Initializing ockam node
Credential obtained from Issuer.
Presenting credentials to Verifier
```

To ensure the example worked, in the verifier console output look for:
```shell
Holder presented credentials.
Credential is valid!
```
