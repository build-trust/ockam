## Auth0 API

Allows clients to enroll their identity with Auth0 authenticator

Worker address: "auth0_authenticator"

Authorization:
**Requires connection via secure channel**

#### Enroll with auth0
Method: POST \
Path: "v0/enroll" \
Request: enroll_request \
Response: ""

Errors:
- 400 - invalid request format
- 400 - invalid token type

Where:
```
enroll_request = {
  1: token_type,
  2: access_token
}

token_type = 0 ;; bearer
access_token = text
```
