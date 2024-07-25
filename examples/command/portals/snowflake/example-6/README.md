# Ockam node as a Snowflake app

![Architecture](diagram.png)

This example shows how to query a private Postgres database from a Snowflake native app.

There are three main steps involved in that setup:

1. Enroll with an Ockam and create the credentials necessary to establish a secure channel between the Snowflake native
   application and Postgres.
2. Setup a Postgres database.
3. Deploy a Snowflake native app which will query the Postgres database.

The communication between Snowflake and Postgres will be mediated by two Ockam nodes, the first one running inside the
Snowflake native application and the second one running alongside the private Postgres database.

## Prerequisites

In order to run this example you need to install the following:

- Ockam,
  with `curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash && source "$HOME/.ockam/env"`.
- [Docker](https://docs.docker.com/get-docker).
- [Snowflake-cli](https://docs.snowflake.com/en/developer-guide/snowflake-cli-v2/installation/installation).
- `envsubst` (via the `gettext` package on [Mac](https://formulae.brew.sh/formula/gettext),
  and [Linux](https://www.gnu.org/software/gettext/gettext.html)).

## Get started with Ockam

[Signup for Ockam](https://www.ockam.io/signup) and then run the following commands on your workstation:

```sh
# Enroll with Ockam Orchestrator.
ockam enroll

# Create an enrollment ticket for the node that will run inside the native application.
export CLIENT_TICKET="$(ockam project ticket --usage-count 1 --expires-in 10h --attribute postgres-client)"

# Create an enrollment ticket for the node that will run alongside the private Postgres database.
export SERVER_TICKET="$(ockam project ticket --usage-count 1 --expires-in 10h --attribute postgres-server --relay postgres)"

# Print the egress allow list for the Ockam project. You will use them later in this example.
export EGRESS_ALLOW_LIST="$(ockam project show --jq .egress_allow_list | sed "s/\"/'/g" | sed "s/\[/(/g" | sed "s/\]/)/g")"
```

### Create the database

First you need to create a version of the SQL creation script containing your Snowflake user name with:

```
export USER_NAME=<your user name here>

cat ./ockam_node/prepare/consumer.sql | envsubst | snow sql --stdin
```

### Build the native application

The native application uses a Docker image starting an Ockam node:

```
docker build --rm --platform linux/amd64 -t ockam_node:on ./ockam_node/application/ockam_node 
```

Then we publish this image to the Snowflake image repository created in the previous section.
First, we get the repository URL:

```sh
# Login
snow spcs image-registry login

# Get the repository URL
export REPOSITORY_URL="$(snow spcs image-repository url ockam_database.ockam_schema.ockam_repository --role ockam)"
```

We tag the image with the repository URL:

```shell
docker tag ockam_node:on $REPOSITORY_URL/ockam_node:on
```

We push the image to the repository:

```shell
docker push $REPOSITORY_URL/ockam_node:on
```

We can run the following command to confirm that the image has been correctly uploaded:

```shell
snow spcs image-repository list-images ockam_database.ockam_schema.ockam_repository --role ockam
```

## Deploy the application

Now we can deploy and instantiate the application:

```shell
snow app run --project ./ockam_node/application
```

If that step is successful you should see a message like:

```shell
Your application object (ockam_node) is now available:
https://app.snowflake.com/HYCWVDM/ekb57526/#/apps/application/OCKAM_NODE
```
