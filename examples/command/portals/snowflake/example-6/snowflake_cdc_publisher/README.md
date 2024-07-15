## Introduction

This directory contains the files necessary to deploy a Snowflake native application

### Files description

| File Name                                     | Purpose                                                                                                                                                                                             |
|-----------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| database.sql                                  | This is a list of instructions to execute in order to create a test database for this example.                                                                                                      |
| application/manifest.yml                      | Defines properties required by the application package. Find more details at the [Manifest Documentation.](https://docs.snowflake.com/en/developer-guide/native-apps/creating-manifest)             
| application/application_setup.sql             | Contains SQL statements that are executed when the cdc_publisher application is installed or upgraded.                                                                                              |
| application/README.md                         | The application README is used to document the application when it is published.                                                                                                                    |
| application/snowflake-cli.yml                 | This file is used by the [Snowflake CLI tool](https://docs.snowflake.com/en/developer-guide/snowflake-cli-v2/index) to interact with your Snowflake account with all relevant prvileges and grants. |
| application/services/spec.yaml                | This specifies the containers which will be deployed as part of the application: cdc_publisher and ockam.                                                                                           |
| application/services/cdc_publisher/Dockerfile | This Docker file is used to create the image used when deploying the cdc_publisher service.                                                                                                         |
| application/services/cdc_publisher/service.py | This Python file contains the code which reads events from Snowflake tables and publishes them to a Kafka broker.                                                                                   |
