

Create everything:

```
make
```

Wait for Compute Pool to be ACTIVE or IDLE:
```
make compute_pool_status
```
---

Once the Compute Pool is active and all the containers are started, run `make test`.

```
» make test
echo 'hello' > hello.txt
echo 'put hello.txt /stage/hello.txt' | sftp -i ./sftp_host_rsa_key -P 2222 foo@127.0.0.1
Connected to 127.0.0.1.
sftp> put hello.txt /stage/hello.txt
Uploading hello.txt to /stage/hello.txt
dest open "/stage/hello.txt": Permission denied
echo 'get /stage/hello.txt hello1.txt' | sftp -i ./sftp_host_rsa_key -P 2222 foo@127.0.0.1
Connected to 127.0.0.1.
sftp> get /stage/hello.txt hello1.txt
Fetching /stage/hello.txt to hello1.txt
hello.txt                                                                                                                                           100%    6     0.1KB/s   00:00
if cmp -s hello.txt hello1.txt; then echo 'PASSED'; else echo 'FAILED'; exit 1; fi
PASSED
rm hello.txt hello1.txt
```

If everything works, you'll see the string `PASSED` in the output.

---

Delete everything:

```
make cleanup
```
