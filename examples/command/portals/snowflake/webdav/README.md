

Create everything:

```
make
```

Wait for Compute Pool to be ACTIVE or IDLE:
```
make compute_pool_status
```

Once the Compute Pool is active and all the containers are started, run `make test`

```
» make test
curl --head http://localhost:8001
HTTP/1.1 200 OK
Date: Thu, 29 Aug 2024 07:32:57 GMT
Server: Apache/2.4.37 (Unix)
Content-Type: text/html;charset=ISO-8859-1
```

Delete everything:

```
make cleanup
```
