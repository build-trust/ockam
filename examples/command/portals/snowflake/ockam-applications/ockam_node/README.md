## Introduction

This directory contains the files necessary to deploy the `ockam_node` Snowflake native application.
This application allows a Snowflake user to:

- Start an Ockam node as a TCP inlet.
- Start an Ockam node as a TCP outlet.
- Start an Ockam node with a general configuration file.

## Files description

| File Name                         | Purpose                                                                                                                                                                                  |
|-----------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| application/ockam_node/Dockerfile | This Docker file is used to create the image used when deploying the ockam_node service.                                                                                                 |
| restart.sh                        | This script rebuilds, redeploys and restarts the application.                                                                                                                            |
| teardown.sh                       | This script deletes the application instance and all its related objects.                                                                                                                | 
| application_setup.sql             | This script creates the Ockam database which hosts the application.                                                                                                                      |
| application/README.md             | The application README is used to document the application when it is published.                                                                                                         |
| application/snowflake.yml         | This file is used by the [Snowflake CLI tool](https://docs.snowflake.com/en/developer-guide/snowflake-cli-v2/index) to upload and start the application.                                 |
| application/sql/setup.sql         | This file creates the objects and functions used by the application                                                                                                                      |
| application/sql/support.sql       | This file creates some additional functions to query the application service status or logs                                                                                              |
| application/sql/post_deploy.sql   | This file provides some commands to run once the application has been started                                                                                                            |
| application/manifest.yml          | Defines properties required by the application package. Find more details at the [Manifest Documentation.](https://docs.snowflake.com/en/developer-guide/native-apps/creating-manifest). |
| application/spec.yml              | This file specifies the container which is be deployed as part of the application.                                                                                                       |
| application/ockam_node/run.sh     | This script starts an Ockam node with a list of arguments specified in .                                                                                                                 |

## Installation

First, we set-up the necessary database and roles to develop and publish an application:

```shell
snow sql -f application_setup.sql
```

Then we need to push the Ockam Docker image to the Snowflake repository.

First, we get the repository URL:

```sh
# Login
snow spcs image-registry login

# Get the repository URL
export REPOSITORY_URL="$(snow spcs image-repository url ockam_database.ockam_schema.ockam_repository --role ockam)"
```

We tag the image with the repository URL:

```shell
docker tag ghcr.io/build-trust/ockam $REPOSITORY_URL/ockam
```

We push the image to the repository:

```shell
docker push $REPOSITORY_URL/ockam
```

We can run the following command to confirm that the image has been correctly uploaded:

```shell
snow spcs image-repository list-images ockam_database.ockam_schema.ockam_repository --role ockam
```

## Deploy the application

Now we can deploy and instantiate the application:

```shell
snow app run --project ./application
```

If that step is successful you should see a message like:

```shell
Your application object (ockam_node) is now available:
https://app.snowflake.com/HYCWVDM/ekb57526/#/apps/application/OCKAM_NODE
```

## Publish the application

Follow the instructions on [this page](https://other-docs.snowflake.com/en/native-apps/provider-publishing-app-package)
to publish the application.
