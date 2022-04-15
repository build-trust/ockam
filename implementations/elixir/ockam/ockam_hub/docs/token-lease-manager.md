# Token Lease Manager

## Introduction

Token Lease Manager is a service powered by Ockam Hub in order to handle the life cycle of Tokens. These tokens will be provided by external, usually cloud, services like InfluxDB.

The main issue of the *Token Cloud providers* is that they do not manage expirations and it is the problem that we have solved.

## Token Lease Manager - Worker

Currently, *Token Lease Manager* is a Ockam Worker which depends on two main pieces:

* *cloud service*: External managed cloud service which provides tokens.
* *storage service*: System where saving the necessary information to revoke tokens when they have expired.

They will be modules that will be passed as arguments under `:cloud_service_module` and `:storage_service_module`.

It will be started if the service `token_lease` is required.

## Cloud Service

Every Cloud Service should `use` the macro `Ockam.TokenLeaseManager.CloudService` and implements the following callbacks:

* *handle_init(options)*: it will be executed during worker's initialization
* *handle_create(cloud_configuration, creation_options)*: It will create the token with the `create_options`. `cloud_configuration` has to contain the necessary data to communicate with the actual cloud service.
* *handle_revoke(cloud_configuration, token_id)*: It will revoke the token with the given `token_id`. `cloud_configuration` has to contain the necessary data to communicate with the actual cloud service.
* *handle_renew(cloud_configuration, token_id)*: It will renew the token with the given `token_id`. `cloud_configuration` has to contain the necessary data to communicate with the actual cloud service. (*not used*).
* *handle_get(cloud_configuration, token_id)*: It will retrieve the token with the given `token_id`. `cloud_configuration` has to contain the necessary data to communicate with the actual cloud service.
* *handle_get_all(cloud_configuration)*: It will retrieve all tokens. `cloud_configuration` has to contain the necessary data to communicate with the actual cloud service.
* *handle_get_address(cloud_configuration)*: It will return the cloud service address. `cloud_configuration` has to contain the necessary data to communicate with the actual cloud service.

## Storage Service

Storage service is aware of tokens are leased and it also have to know about the used cloud service.

Every Storage Service should `use` the macro `Ockam.TokenLeaseManager.StorageService` and implements the following callbacks:

* *handle_init({cloud_service, cloud_service_address})*: Storage service must know the name and the address of the cloud service that will work with. These data is used to differentiate between users.
* *handle_save(storage_conf, lease)*: It saves the given `lease`. `storage_conf` has to contain the necessary data to communicate with the actual storage service.
* *handle_get(storage_conf, lease_id)*: It retrieves the lease with the given `lease_id`. `storage_conf` has to contain the necessary data to communicate with the actual storage service.
* *handle_remove(storage_conf, lease_id)*: It removes the lease with the given `lease_id`. `storage_conf` has to contain the necessary data to communicate with the actual storage service.
* *handle_get_all(storage_conf)*: It retrieves all leases. `storage_conf` has to contain the necessary data to communicate with the actual storage service.

## Encoding

The messages will be encoded using JSON.

## Token Lease

Token Lease data structure fields will be the following ones:

* **value**: actual token
* **id**: token's identifier
* **ttl**: time to live
* **tags**: related to the cloud service
* **issued**: when the token was issued
* **renewable**: it indicates if the token should be renewed (not used)

**InfluxDB's token example:**

```json
{
    "value": "Cui5qBH9ItFvzKJqaCHedZ0ZY_fZnAr63ajnZ4DX1zz6_Q-eJUBmGAi-o_I2j8P_9hbyxqG02NlkPLjP1qRudQ==",
    "ttl": 60000,
    "tags": [],
    "renewable": false,
    "issued": "2021-08-26T08:59:17.991429689Z",
    "id": "080c6b2f99f7e000"
}
```

## API

The API exposes methods to create, read and revoke tokens. These methods should be specified in the sent message under the mandatory field `action`.

The fields of the response will be the following ones:

* **result**: "success" or "failure" (mandatory)
* **lease**: lease data (optional)
* **message**: usually, error message (optional)

### Creation

The message will contain the action `create` and the options related to the used cloud service. Options also include the time to live `ttl` in milliseconds. The example is for InfluxDB.

**Example:**

```json
{
    "action": "create",
    "options": {
        "ttl": 60000,
        "orgID": "217eba198af721b8",
        "permissions": [
            {
                "action": "read",
                "resource": {
                    "type": "authorizations"
                }
            }
        ]
    }
}
```

**Response Example:**

```json
{
    "result": "success",
    "lease": {
        "value": "Cui5qBH9ItFvzKJqaCHedZ0ZY_fZnAr63ajnZ4DX1zz6_Q-eJUBmGAi-o_I2j8P_9hbyxqG02NlkPLjP1qRudQ==",
        "ttl": 60000,
        "tags": [],
        "renewable": false,
        "issued": "2021-08-26T08:59:17.991429689Z",
        "id": "080c6b2f99f7e000"
    }
}
```

### Reading

The message will contain the action `get` and the `token_id`.

**Example:**

```json
{
    "action": "get",
    "token_id": "080c6f2b3af7e000"
}
```

**Response Example:**

```json
{
    "result": "success",
    "lease": {
        "value": "lsHqFqd7qf6JuW9A0zQI6FZc29FmGxNaxRe5VbzZX5-GxIAQGX7HX_uTOdlwWNgcyCi0BlofvXOC6vdDHPMAmg==",
        "ttl": 60000,
        "tags": [],
        "renewable": false,
        "issued": "2021-08-26T09:16:42.091069552Z",
        "id": "080c6f2b3af7e000"
    }
}
```

### Revoking

The message will contain the action `revoke` and the `token_id`.

**Example:**

```json
{
    "action": "revoke",
    "token_id": "080c6fde1df7e000"
}
```

**Response Example:**

```json
{
    "result": "success"
}
```
