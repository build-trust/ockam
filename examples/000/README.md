### Create an account

To get started with Ockam AI, your first step
is to [sign up](https://ockam.io/signup) for an account. This account
will give you the ability to deploy and manage AI applications in Ockam AI.

### Install

Your next step is to install the `ockam` command. This command-line
tool lets you deploy and manage AI applications in Ockam AI.

Copy and run the following on your terminal, it will download and
set up the `ockam` command quickly. It will also add the right environment
variables to make the command ready for immediate use.

```sh
curl -sSfL install.command.ockam.io | bash && source "$HOME/.ockam/env"
````

### Get code from a template

To help you hit the ground running, we provide pre-configured
templates to bootstrap your development.

Run the following, in a new empty directory to initialize
the code for your Zone:

```sh
ockam zone init build-trust/ockam/examples/000
```

A zone can run many containers as defined by the `ockam.yaml` file.
The above template starts only one container, called `main` that is based
on the docker image defined at `images/main`. This container runs
a python app defined in `images/main/main.py`.

### Populate secrets

The `main` python app provides an HTTP API. This API is private and the
API KEY needed to access this API must be provided in `secrets.yaml`.
The included `secrets.example.yaml` file shows the
required secret structure.

Create `secrets.yaml` with a strong random 32-byte key:

```sh
sed "s|your_api_key_here|$(openssl rand -hex 32)|" secrets.example.yaml > secrets.yaml
```

### Make a temporary deployment

While actively developing your Zone, you can run a temporary
deployment that automatically reloads on changes.

The following command deploys your zone and watches your source
files for edits, making development fast and iterative. This deployment
is ephemeral, when you press `Ctrl+C` to exit the command, your zone
will be immediately deleted.

> [!IMPORTANT]
> Ensure you have [Docker](https://www.docker.com/get-started/) installed
> and running on your workstation before running the following command.
> It uses docker to build container images.

```sh
ockam --rm --watch
```

This will output a URL to the http server on the main-pod in your zone.
Use this URL to send a query to the deployed app.

```sh
curl -s -X POST https://fccdfd97526a97da0f0262da60861ce8-example000.ai.ockam.network/analyses \
  -H "X-API-KEY: 2344fe90d2d706d8a4495fdcb517fbcbefe901f12424a33efa920a0bef26aed0" \
  -H "Content-Type: application/json" \
  -d '{"items":["hello", "bye"]}'
```

Replace the URL in `URL/analyses` in the curl command above
with your main-pod's URL. Replace the API KEY header with the API KEY that was
generated in your `secrets.yaml` file.

The example app that we've deployed accepts a list of items. Each item can
be simple word or a complex document. The app uses an Ockam Agent to analyize
each item in parallel and responds with a translation of that item.

### Make a permanent deployment

Once your Zone is ready for production, deploy it permanently:

```sh
ockam zone deploy
```

This will create a long-running Zone in your Cluster, managed by Ockam.

### Set up automatic deployments

You may commit your code to Github.

The above init template includes code for a GitHub Actions
Workflow that will automatically deploy you zone.
Whenever you push updates to the main branch, the action will trigger and
redeploy your Zone, to ensure that your production environment
is always up to date.

Create a github actions environment secret called `OCKAM_IDENTITY` and
set its value copied to your clipboard by
running `ockam identity export | pbcopy` on your workstation.

Create a github actions environment secret called `OCKAM_ZONE_SECRETS` and
set its value copied to your clipboard by
running `cat secrets.yaml | pbcopy` on your workstation.
