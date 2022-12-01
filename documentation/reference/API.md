## Ockam API workers

Ockam Workers serve as actors being able to handle and send Ockam Messages.
Ockam Routing protocol only describes unidirectional messages with very simple
message structure. Internal structure of the message payload is up to the worker
implementation.

A common messaging pattern, especially with the online services is request-response,
which can be implemented with Ockam Messages.

API workers provide a relatively simple enough framework for workers
to implement request-response messaging pattern.

API worker communication consists of the API worker and the Client worker exchanging
Request and Response messages.

API worker receives Requests and replies with Responses.

Client worker sends a Request and waits for a Response.


### Request and Response format

Request and response payloads consist of a CBOR encoded header and a binary body.

Headers are described in the following CDDL schema:

```
request_header = {
    ?0: 7586022,
     1: id,
     2: path,
     3: method,
     4: has_body
}

response_header = {
    ?0: 9750358,
     1: id,
     2: request_id,
     3: status,
     4: has_body
}

error = {
    ?0: 5359172,
    ?1: path,
    ?2: method,
    ?3: message
}

id = uint
request_id = uint
path = text ;; additional metadata about the request
has_body = bool ;; if true - header is followed by the body

method = 0 ;; GET
       / 1 ;; POST
       / 2 ;; PUT
       / 3 ;; DELETE
       / 4 ;; PATCH

status = uint
;; 200 OK
;; 400 Bad request
;; 401 Unauthorized
;; 404 Not found
;; 405 Method not allowed
;; 409 Resource exists
;; 500 Internal server error
;; 501 Not implemented
```

The format mimics HTTP request/response structures.

### API request/response reference format

To describe an API, we can use the following format:

````
### Cats API

Worker address: "cats"

#### See the cats face
Method: GET \
Path: ":cat_name/face" \
Request: "" \
Response: cats_face_response

Errors:
- 404 - cat not found

Where:
```
cats_face_response = {
  1: eyes_colour,
  2: whiskers_count
}
eyes_colour = text
whiskers_count = uint
```

#### Pet the cat
Method: PUT \
Path: ":cat_name/pet" \
Request: pet_cat_request \
Response: purr_response

Errors:
- 409 - cat is busy
- 401 - not your cat


Where:
```
pet_cat_request = {
  1: way,
  2: part
}
way: text
part: text

purr_response = {
  1: volume_db
}
volume_db = uint
```
````

`Request` and `Response` describe the body format in CDDL,
it's recommended to use CBOR for request and response bodies.

Note: `\ ` at the line ends are there to format markdown line breaks properly

More examples of API docs can be found in the [API reference folder](./api)


