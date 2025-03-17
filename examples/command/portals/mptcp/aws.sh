#!/usr/bin/env bash
set -ex

run() {
    ticket="$1"
    type="$2"
    number_of_subflows="$3"
    machine_type="m8g.48xlarge"

    # ----------------------------------------------------------------------------------------------------------------
    # CREATE NETWORK

    # Create a new VPC and tag it.
    vpc_id=$(aws ec2 create-vpc --cidr-block 10.0.0.0/16 --query 'Vpc.VpcId')
    aws ec2 create-tags --resources "$vpc_id" --tags "Key=Name,Value=${name}-vpc"

    # Create an Internet Gateway and attach it to the VPC.
    gw_id=$(aws ec2 create-internet-gateway --query 'InternetGateway.InternetGatewayId')
    aws ec2 attach-internet-gateway --vpc-id "$vpc_id" --internet-gateway-id "$gw_id"

    # Create a Route Table and a route to the Internet through the Gateway.
    rtb_id=$(aws ec2 create-route-table --vpc-id "$vpc_id" --query 'RouteTable.RouteTableId')
    aws ec2 create-route --route-table-id "$rtb_id" --destination-cidr-block 0.0.0.0/0 --gateway-id "$gw_id"

    # Create a Subnet and associate the Route Table.
    az=$(aws ec2 describe-availability-zones --query "AvailabilityZones[-1].ZoneName")
    subnet_id=$(aws ec2 create-subnet --vpc-id "$vpc_id" --cidr-block 10.0.0.0/24 \
        --availability-zone "$az" --query 'Subnet.SubnetId')
    aws ec2 modify-subnet-attribute --subnet-id "$subnet_id" --map-public-ip-on-launch
    aws ec2 associate-route-table --subnet-id "$subnet_id" --route-table-id "$rtb_id"

    # Create a Security Group.
    sg_id=$(aws ec2 create-security-group --group-name "${name}-sg" --vpc-id "$vpc_id" --query 'GroupId' \
        --description "Allow ssh ingress and all egress")
    # my_ip=$(curl -s http://checkip.amazonaws.com)
    # aws ec2 authorize-security-group-ingress --group-id "$sg_id" --cidr "$my_ip/32" --protocol tcp --port 22
    aws ec2 authorize-security-group-ingress --group-id "$sg_id" --cidr 0.0.0.0/0 --protocol tcp --port 0-65535

    # ----------------------------------------------------------------------------------------------------------------
    # CREATE INSTANCE
    ami_id=$(aws ec2 describe-images --owners 137112412989 --query "Images | sort_by(@, &CreationDate) | [-1].ImageId" \
                --filters "Name=name,Values=al2023-ami-2023.*"  "Name=architecture,Values=arm64" \
                          "Name=virtualization-type,Values=hvm" "Name=root-device-type,Values=ebs")

    aws ec2 create-key-pair --key-name "${name}-key" --query 'KeyMaterial' > key.pem
    chmod 400 key.pem

cat <<EOF > user_data.sh
#!/bin/bash
set -ex

curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash -s -- --install-path /opt/ockam
chmod +x /opt/ockam/bin/ockam
cat /opt/ockam/env > /etc/profile.d/ockam.sh

echo "$ticket" > ticket
chmod +r ticket

# sudo yum install iperf3 -y
sudo yum groupinstall -y "Development Tools"
git clone https://github.com/esnet/iperf.git

pushd iperf
  ./configure; make -j 4; sudo make install
popd
EOF

    instance_id=$(aws ec2 run-instances --image-id "$ami_id" --instance-type "$machine_type" \
        --subnet-id "$subnet_id" --security-group-ids "$sg_id" \
        --user-data file://user_data.sh --key-name "${name}-key" --query 'Instances[0].InstanceId')
    aws ec2 create-tags --resources "$instance_id" --tags "Key=Name,Value=${name}-ec2-instance"
    aws ec2 wait instance-status-ok --instance-ids "$instance_id"

    netwrok_interface_id=$(aws ec2 describe-instances --instance-ids $instance_id \
        --query "Reservations[0].Instances[0].NetworkInterfaces[0].NetworkInterfaceId")

    # Enable ENA Express
    aws ec2 modify-network-interface-attribute --network-interface-id $netwrok_interface_id --ena-srd-specification EnaSrdEnabled=true

    public_ip=$(aws ec2 describe-instances --instance-ids "$instance_id" \
        --query 'Reservations[0].Instances[0].PublicIpAddress')

    scp -o StrictHostKeyChecking=no -i ./key.pem ../ockam "ec2-user@$public_ip:ockam"
    ssh -o StrictHostKeyChecking=no -i ./key.pem "ec2-user@$public_ip" \
        'bash -s' << EOS

            sudo chmod u+x ockam
            sudo mv -f ockam /opt/ockam/bin/ockam
EOS

    additional_mac_addrs=()

    if [[ -z "$number_of_subflows" ]]; then
            number_of_subflows=0
    fi

    if [ "$type" == "inlet" ]; then
        for ((i=0; i<$number_of_subflows; i++)); do
            eni_id=$(aws ec2 create-network-interface --subnet-id $subnet_id --groups $sg_id --description "Secondary interface $i" --query "NetworkInterface.NetworkInterfaceId")
            aws ec2 wait network-interface-available --network-interface-ids $eni_id
            aws ec2 create-tags --resources "$eni_id" --tags "Key=Name,Value=${name}-secondary-interface-$i"

            mac_address=$(aws ec2 describe-network-interfaces --network-interface-ids $eni_id --query "NetworkInterfaces[-1].MacAddress")

            ip_id=$(aws ec2 allocate-address --domain vpc --query "AllocationId")
            aws ec2 create-tags --resources "$ip_id" --tags "Key=Name,Value=${name}-additional-ip-$i"

            aws ec2 associate-address --network-interface-id $eni_id --allocation-id $ip_id

            aws ec2 attach-network-interface --network-interface-id $eni_id --instance-id $instance_id --device-index $((i+1))

            additional_mac_addrs+=($mac_address)
        done
    fi

    if [ "$type" == "inlet" ]; then
        ssh -o StrictHostKeyChecking=no -i ./key.pem "ec2-user@$public_ip" \
            'bash -s' << EOS

                endpoints=\$(sudo ip mptcp endpoint show | awk '{print \$3}')

                for id in \$endpoints; do
                    sudo ip mptcp endpoint delete id \$id
                done

                read -r -a additional_mac_addrs <<< "${additional_mac_addrs[@]}"
                for mac in "\${additional_mac_addrs[@]}"; do
                    dev=\$(ip -o link show | awk -v mac="\$mac" '\$0 ~ mac {print \$2}' | sed 's/://')
                    ip=\$(ip -o -4 addr show \$dev | awk '{print \$4}' | cut -d/ -f1)

                    sudo ip mptcp endpoint add \$ip dev \$dev subflow
                done
EOS
    fi

    additional_ports=()

    if [ "$type" == "outlet" ]; then
        for ((i=0; i<$number_of_subflows; i++)); do
            additional_ports+=($((i+6000)))
        done
    fi

    if [ "$type" == "outlet" ]; then
        ssh -o StrictHostKeyChecking=no -i ./key.pem "ec2-user@$public_ip" \
            'bash -s' << EOS

                dev=\$(ip -o link show | awk -F': ' '\$2 != "lo" {print \$2}' | head -n 1)

                private_ip=\$(ip -4 addr show \$dev | grep -oP '(?<=inet\s)\d+(\.\d+){3}')

                sudo ip addr add $public_ip/32 dev \$dev

                read -r -a additional_ports <<< "${additional_ports[@]}"
                for port in "\${additional_ports[@]}"; do
                    sudo ip mptcp endpoint add $public_ip dev \$dev signal port \$port
                done

                sudo sh -c "echo 1 > /proc/sys/net/ipv4/ip_forward"

                sudo yum install -y iptables iptables-services

                sudo systemctl enable iptables

                sudo tee /etc/sysconfig/iptables <<EOF
*filter
:INPUT ACCEPT [0:0]
:FORWARD ACCEPT [0:0]
:OUTPUT ACCEPT [0:0]
COMMIT
*nat
:PREROUTING ACCEPT [0:0]
:INPUT ACCEPT [0:0]
:OUTPUT ACCEPT [0:0]
:POSTROUTING ACCEPT [0:0]
EOF

                for port in "\${additional_ports[@]}"; do
                    sudo tee -a /etc/sysconfig/iptables <<EOF
-A PREROUTING -d \$private_ip -p tcp -m tcp --dport \$port -j DNAT --to-destination $public_ip:\$port
EOF
                done

                sudo tee -a /etc/sysconfig/iptables <<EOF
COMMIT
EOF

                sudo systemctl start iptables
EOS
    fi


    ssh -o StrictHostKeyChecking=no -i ./key.pem "ec2-user@$public_ip" \
        'bash -s' << EOS
            sudo ip mptcp limits set subflows 4
            sudo ip mptcp limits set add_addr_accepted 8

            sudo sysctl -w net.core.rmem_max=80000000
            sudo sysctl -w net.core.wmem_max=80000000

            sudo sysctl -w net.ipv4.tcp_rmem="4096 7000000 70000000"
            sudo sysctl -w net.ipv4.tcp_wmem="4096 7000000 70000000"
EOS

    echo "ssh -o StrictHostKeyChecking=no -i ./key.pem ec2-user@$public_ip"
}

cleanup() {
    # ----------------------------------------------------------------------------------------------------------------
    # DELETE INSTANCE

    instance_ids=$(aws ec2 describe-instances --filters "Name=tag:Name,Values=${name}-ec2-instance" "Name=instance-state-name,Values=pending,running,shutting-down,stopping,stopped" \
        --query "Reservations[*].Instances[*].InstanceId")
    for i in $instance_ids; do
        aws ec2 terminate-instances --instance-ids "$i"
        aws ec2 wait instance-terminated --instance-ids "$i"
    done

    # ----------------------------------------------------------------------------------------------------------------
    # DELETE NETWORK INTERFACE
    network_ids=$(aws ec2 describe-network-interfaces --filters "Name=tag:Name,Values=${name}-secondary-interface*" \
        --query "NetworkInterfaces[*].NetworkInterfaceId")
    for i in $network_ids; do
        aws ec2 delete-network-interface --network-interface-id "$i"
    done

    # ----------------------------------------------------------------------------------------------------------------
    # DELETE ADDITIONAL IP
    additional_ip_ids=$(aws ec2 describe-addresses --filters "Name=tag:Name,Values=${name}-additional-ip*" \
        --query "Addresses[*].AllocationId")
    for i in $additional_ip_ids; do
        aws ec2 release-address --allocation-id "$i"
    done

    if aws ec2 describe-key-pairs --key-names "${name}-key" &>/dev/null; then
        aws ec2 delete-key-pair --key-name "${name}-key"
    fi
    rm -f key.pem user_data.sh

    # ----------------------------------------------------------------------------------------------------------------
    # DELETE NETWORK

    vpc_ids=$(aws ec2 describe-vpcs --query 'Vpcs[*].VpcId' --filters "Name=tag:Name,Values=${name}-vpc")

    for vpc_id in $vpc_ids; do
        internet_gateways=$(aws ec2 describe-internet-gateways --query "InternetGateways[*].InternetGatewayId" \
            --filters Name=attachment.vpc-id,Values="$vpc_id")
        for i in $internet_gateways; do
            aws ec2 detach-internet-gateway --internet-gateway-id "$i" --vpc-id "$vpc_id"
            aws ec2 delete-internet-gateway --internet-gateway-id "$i"
        done

        subnet_ids=$(aws ec2 describe-subnets --query "Subnets[*].SubnetId" --filters Name=vpc-id,Values="$vpc_id")
        for i in $subnet_ids; do aws ec2 delete-subnet --subnet-id "$i"; done

        route_tables=$(aws ec2 describe-route-tables  --filters Name=vpc-id,Values="$vpc_id" \
            --query 'RouteTables[?length(Associations[?Main!=`true`]) > `0` || length(Associations) == `0`].RouteTableId')
        for i in $route_tables; do aws ec2 delete-route-table --route-table-id "$i" || true; done

        security_groups=$(aws ec2 describe-security-groups --filters Name=vpc-id,Values="$vpc_id" \
            --query "SecurityGroups[?!contains(GroupName, 'default')].[GroupId]")
        for i in $security_groups; do aws ec2 delete-security-group --group-id "$i"; done

        if aws ec2 describe-vpcs --vpc-ids "$vpc_id" &>/dev/null; then
            aws ec2 delete-vpc --vpc-id "$vpc_id"
        fi
    done
}

export AWS_PAGER="";
export AWS_DEFAULT_OUTPUT="text";

user=""
command -v sha256sum &>/dev/null && user=$(aws sts get-caller-identity | sha256sum | cut -c 1-20)
command -v shasum &>/dev/null && user=$(aws sts get-caller-identity | shasum -a 256 | cut -c 1-20)
export name="ockam-quick-$user"

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then cleanup; else run "$1" "$2" "$3"; fi
