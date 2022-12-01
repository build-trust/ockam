## Credential exchange API

Allows client nodes to present credentials (one-way exchange)

Worker address: "credentials"

Implemented in `Ockam.Services.API.CredentialExchange`


#### Present credential
Method: POST \
Path: "actions/present" \
Request: Credential \
Response: ""

Errors:
- 400 - credential is invalid
- 400 - secure channel required

Where `Credential` is a binary with credential received from credential exchange.
