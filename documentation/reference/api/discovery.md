## Discovery API

Shows info about services running on the node

Worker address: "discovery"

Implemented in `Ockam.Services.API.Discovery`

**NOTE: this API is a work in progress**

#### List services
Method: GET
Path: ""
Request: ""
Response: [+ service_info]

#### Show service
Method: GET
Path: ":service_id"
Request: ""
Response: service_info

#### Register service
Method: PUT
Path: ":service_id"
Request: service_info
Response: ""

Errors:
400 - cannot decode service_info
405 - method not allowed

Some backends do not support service registration and will always return status 405

Where:
```
service_info = {
  1: id,
  2: route,
  3: metadata
}

id = text
route = {
  type: uint,
  value: binary
}
metadata = {* binary => binary}
```



