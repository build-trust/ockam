## Spaces API

Provides control over orchestrator spaces.

Worker address: "spaces"

Authorization:
- Requires connection via secure channel
- Identity needs to be enrolled to the Orchestrator Controller via [Auth0](./auth0.md)

#### List spaces
Method: GET
Path: "/v0"
Request: ""
Response: `[+ space]`

#### Create space
Method: POST
Path: "/v0"
Request: create_request
Response: space

Errors:
401 - current user does not have permission to create a space
409 - name should be unique
400 - invalid name, it should match the regexp: `^([[:alnum:]])+([-_\.]?[[:alnum:]])*$`
400 - invalid request format

#### Show space
Method: GET
Path: /v0/:space_id
Request: ""
Response: space

Errors:
404 - not found
401 - current user does not have permission to show the space

#### Update space
Mathod: PUT
Path: /v0/:space_id
Request: update_request
Response: space

Errors:
404 - not found
401 - current user does not have permission to update the space
409 - name should be unique
400 - invalid name, it should match the regexp: `^([[:alnum:]])+([-_\.]?[[:alnum:]])*$`
400 - invalid request format

**WARNING**: updating spaces is not recommended. This API may be removed in the future

#### Delete space
Method: DELETE
Path: /v0/:space_id
Request: ""
Response: ""

Errors:
404 - not found
401 - current user does not have permission to delete the space

Where:
```
space = {
  1: id,
  2: name,
  3: users

}

create_request = update_request = {
  1: name,
  2: users
}

id = text
name = text
users = [+ text]


```

