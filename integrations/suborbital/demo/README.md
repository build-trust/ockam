1. Run outlet docker image on Control Plane node:

    ```sh
    docker run ghcr.io/build-trust/ockam/tcp_outlet:latest suborbital.node.ockam.network:4000 control_plane host.docker.internal:4001
    ```

    Where arguments describe following:
    - Ockam's cloud node address
    - Alias that is used to identify Control Plane node
    - Address of tcp service running on Control Plane node that will receive connections from the Outlet

2. Run inlet binary on Edge Data Plane node(s):

    ```sh
    ./ockam_tcp_inlet suborbital.node.ockam.network:4000 control_plane 127.0.0.1:4002
    ```

    Where arguments describe following:
    - Ockam's cloud node address
    - Alias that is used to identify Control Plane node
    - Bind address that Inlet will listen on

3. Run test server on Control Plane

    ```sh
    pushd $(mktemp -d 2>/dev/null || mktemp -d -t 'tmpdir') &>/dev/null; python3 -m http.server --bind 0.0.0.0 4001; popd
    ```

4. Test TCP connection from Edge Data Plane

    ```sh
    curl http://127.0.0.1:4002
    ```
