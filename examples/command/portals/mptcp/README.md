## MPTCP

This example shows how to create a TCP Portal that leverages MPTCP to achieve up to 25Gbits/s throughput between different AWS accounts.

## Usage

 - Create a working directory, place ockam binary built for aarch64-unknown-linux-gnu in release mode (by default script installs latest released version of Ockam, but expects a manually built binary in the working directory that would be used to replace the released version for easier debugging)
 - Use 2 different terminal tabs that logged in into 2 different AWS accounts (e.g. use `export AWS_PROFILE="{PROFILE_NAME}"`)
 - Preferably, use the same AWS region to experience the best throughput (e.g., use `export AWS_REGION="{REGION}"`)
 - Create 2 subdirectories for 2 AWS accounts and navigate to them in corresponding terminal tabs
 - Use the script to create the outlet ec2 machine on the 1st account:
    ```
   aws.sh "{ENROLLMENT TICKET}" inlet
   ```
 - Use the script to create the inlet ec2 machine on the 2nd account. `8` refers to the number of additional ports that the server will advertise as available for MPTCP subflows:
   ```
   aws.sh "{ENROLLMENT TICKET}" outlet 8
   ```
 - Use printed full ssh command to connect to both ec2 machines
 - It's recommended to adjust few related variables to achieve the best throughput, e.g.:
   - Ockam settings: `export OCKAM_OPENTELEMETRY_EXPORT=false`, `OCKAM_TCP_PORTAL_PAYLOAD_LENGTH=400000` (see `env_info.txt` for the full list)
   - Kernel settings (these are already part of the script)
     ```
     sudo sysctl -w net.core.rmem_max=80000000
     sudo sysctl -w net.core.wmem_max=80000000

     sudo sysctl -w net.ipv4.tcp_rmem="4096 7000000 70000000"
     sudo sysctl -w net.ipv4.tcp_wmem="4096 7000000 70000000"
     ```
 - Run the following to create the outlet on the outlet machine:
   ```
   ockam node create --foreground --enable-mptcp \
        --enrollment-ticket "$(cat /ticket)" \
        --configuration '{
            "tcp-listener-address": "0.0.0.0:5555",
            "tcp-outlet": {
                "to": "localhost:5000"
            }
        }'
   ```
 - Run the following command to create the inlet on the inlet machine:
   ```
   ockam node create --foreground \
        --enrollment-ticket "$(cat /ticket)" \
        --configuration '{
            "tcp-inlet": {
                "from": "0.0.0.0:4000",
                "to": "/ip4/{PUBLIC_IP_OF_THE_SECOND_EC2}/mptcp/5555/secure/api/service/outlet",
            }
        }'
   ```
   - Now the Portal is ready for throughput testing. The scrit will preinstall iperf3 for that (it's also possible to measure the throughput directly without the portal, the preinstalled version of iperf3 has MPTCP support via adding `-m` flag)
