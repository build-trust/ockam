# Build End-to-End Encrypted and Secure Messaging Channels

In this step-by-step guide we’ll learn how to build mutually-authenticated, end-to-end encrypted,
secure messaging channels that protect en-route messages against eavesdropping, tampering, and forgery.

Data, within modern distributed applications, are rarely exchanged over a single point-to-point
transport connection. Application messages routinely flow over complex, multi-hop, multi-protocol
routes — _across data centers, through queues and caches, via gateways and brokers_ — before reaching
their end destination.

Transport layer security protocols are unable to protect application messages because their protection
is constrained by the length and duration of the underlying transport connection. Ockam is a collection of
programming libraries (in Rust and Elixir) that make it simple for our applications to guarantee end-to-end
integrity, authenticity, and confidentiality of data.

We no longer have to implicitly depend on the defenses of every machine or application within the same,
usually porous, network boundary. Our application's messages don't have to be vulnerable at every point,
along their journey, where a transport connection terminates.

Instead, our application can have a strikingly smaller vulnerability surface and easily make
_granular authorization decisions about all incoming information and commands._

Let's build mutually-authenticated, end-to-end protected communication between distributed applications:

### Setup

To reduce friction and focus the attention on learning, we recommend the usage of a Docker container for the learning exercise. To learn how to get started with Docker, please visit the [Get Started With Docker](https://docs.docker.com/get-docker/) documentation.

This command may take a few minutes the first time you invoke it:

```
docker run --rm -it -e HOST_USER_ID=$(id -u) --name ockam-learn  ghcr.io/build-trust/ockam-builder:latest bash
```

Upon completion, you will be placed inside the `/work` folder of the container. Next, add a text editior for editing files.

```
apt update && apt install nano
```

**NOTE**: If you do not want to use a container for the learning excercise then you will need to install Rust locally. If you don't have it, please [install](https://www.rust-lang.org/tools/install) the latest version of Rust. Only do this step if you chose to not use the learning container.

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Next, create a new cargo project to get started:

```
cargo new --lib hello_ockam && cd hello_ockam && mkdir examples &&
  echo 'ockam = "*"' >> Cargo.toml && cargo build
```

You will need a total of three terminal windows and sessions with the learning container. Go ahead and create all three sessions now using the following command.

```
docker exec --workdir /work/hello_ockam -it ockam-learn bash
```

If the above instructions don't work on your machine, please
[post a question](https://github.com/build-trust/ockam/discussions/1642),
we would love to help.

### Step-by-step

<ul>
<li><a href="./get-started/01-node#readme">01. Node</a></li>
<li><a href="./get-started/02-worker#readme">02. Worker</a>
<li><a href="./get-started/03-routing#readme">03. Routing</a></li>
<li><a href="./get-started/04-transport#readme">04. Transport</a></li>
<li><a href="./get-started/05-secure-channel#readme">05. Secure Channel</a></li>
</ul>

<div style="display: none; visibility: hidden;">
<hr><b>Next:</b> <a href="./get-started/01-node#readme">01. Node</a>
</div>

### Clean-up

You may exit from the learning containers by pressing the following keys, `CTRL+C`, `CTRL+D` or type `exit` in the terminal. You may also close the terminal windows.
