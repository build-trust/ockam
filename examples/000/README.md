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
templates to bootstrap your development. Run the following to initialize
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

> [!IMPORTANT]
> Ensure you have [Github CLI](https://cli.github.com/manual/) installed and
> running on your workstation before running the following commands.

```sh
gh repo create YOUR_REPO_NAME --private --confirm
gh secret set OCKAM_IDENTITY --env production --body "$(ockam identity export)"
gh secret set OCKAM_ZONE_SECRETS --env production --body "$(cat secrets.yaml)"

git init
git add .
git commit -m "Initial commit"
git branch -M main
git remote add origin "git@github.com:YOUR_ORG_NAME/YOUR_REPO_NAME.git"
git push -u origin main
```
