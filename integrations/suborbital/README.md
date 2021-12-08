# suborbital_and_ockam


1. Start Atmo and Ockam Outlet

```
docker-compose -f docker-compose-remote.yaml up
```

This will print a FORWARDING_ADDRESS for this outlet on Ockam Hub. Copy it.

2. Start Ockam Inlet and Sat

```
FORWARDING_ADDRESS=1c1bace60cd0c04b docker-compose -f docker-compose.yaml up
```

Replace `1c1bace60cd0c04b` here with address from step 1.
