# suborbital_and_ockam

---
Start Atmo and Ockam Outlet

```
docker-compose -f docker-compose-ockam-tcp-outlet-atmo.yaml up
```

This will print a FORWARDING_ADDRESS for this outlet on Ockam Hub. Copy it.

---
Start Ockam Inlet and Sat

```
FORWARDING_ADDRESS=FWD_05ea353a2d7b8261 PORT=4001 docker-compose -f docker-compose-ockam-tcp-inlet-sat.yaml up
```

Replace `FWD_05ea353a2d7b8261` here with address from step 1.

---
Send a request

```
curl -d "world" 127.0.0.1:8080/hello
```
