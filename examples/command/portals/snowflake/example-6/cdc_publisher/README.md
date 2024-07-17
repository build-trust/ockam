## Introduction

This directory contains the files necessary to deploy a Snowflake native application

### Files description

| File Name                                     | Purpose                                                                                                                                                                                              |
|-----------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| database.sql                                  | This is a list of instructions to execute in order to create a test database for this example.                                                                                                       |
| application/services/README.md                | The application README is used to document the application when it is published.                                                                                                                     |
| application/snowflake.yml                     | This file is used by the [Snowflake CLI tool](https://docs.snowflake.com/en/developer-guide/snowflake-cli-v2/index) to interact with your Snowflake account with all relevant privileges and grants. |
| application/services/application_setup.sql    | Contains SQL statements that are executed when the cdc_publisher application is installed or upgraded.                                                                                               |
| application/services/post_deploy.sql          | This file provides some commands to run once the application has been started                                                                                                                        |
| application/services/manifest.yml             | Defines properties required by the application package. Find more details at the [Manifest Documentation.](https://docs.snowflake.com/en/developer-guide/native-apps/creating-manifest).             |
| application/services/spec.yml                 | This file specifies the containers which will be deployed as part of the application: cdc_publisher and ockam.                                                                                       |
| application/services/cdc_publisher/Dockerfile | This Docker file is used to create the image used when deploying the cdc_publisher service.                                                                                                          |
| application/services/cdc_publisher/service.py | This Python file contains the code which reads events from Snowflake tables and publishes them to a Kafka broker.                                                                                    |
| application/services/ockam_inlet/Dockerfile   | This Docker file is used to create the image used when deploying the ockam-inlet service.                                                                                                            |
| application/services/ockam_inlet/run.sh       | This script starts an Ockam inlet node.                                                                                                                                                              |
