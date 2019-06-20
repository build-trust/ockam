The `ockam` command is a useful tool to interact with the Ockam Network. You can install it for your
operating system from our [release bundles](https://github.com/ockam-network/ockam/releases).

If you are on Mac or Linux, you can also use this simple
[downloader script](https://github.com/ockam-network/ockam/godownloader-ockam.sh):

```
curl -L https://git.io/fhZgf | sh
```
This will download the binary to `./bin/ockam` in your current directory. It is self-contained, so you can copy it to
somewhere more convenient in your system path, for example:

```
cp ./bin/ockam /usr/local/bin/
```

Once the command is in your path, you can run:

```
ockam --version
```

Next you can run:
```
ockam register
```
which will generate a unique Ockam decentralized identifier for
your computer and register that entity on the Ockam TestNet.
