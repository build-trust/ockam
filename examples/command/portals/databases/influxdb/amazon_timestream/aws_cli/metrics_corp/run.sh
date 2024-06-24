#!/usr/bin/env bash
set -ex

run() {
    enrollment_ticket="$1"

    region=$(aws ec2 describe-availability-zones --query 'AvailabilityZones[0].[RegionName]')
    check_if_timestream_is_available_in_the_configured_region "$region"

    # ----------------------------------------------------------------------------------------------------------------
    # CREATE NETWORK

    # Create a new VPC and tag it.
    vpc_id=$(aws ec2 create-vpc --cidr-block 10.0.0.0/16 --query 'Vpc.VpcId')
    aws ec2 create-tags --resources "$vpc_id" --tags "Key=Name,Value=${name}-vpc"
    aws ec2 modify-vpc-attribute --vpc-id "$vpc_id" --enable-dns-support '{"Value":true}'
    aws ec2 modify-vpc-attribute --vpc-id "$vpc_id" --enable-dns-hostnames '{"Value":true}'

    # Create an Internet Gateway and attach it to the VPC.
    gw_id=$(aws ec2 create-internet-gateway --query 'InternetGateway.InternetGatewayId')
    aws ec2 attach-internet-gateway --vpc-id "$vpc_id" --internet-gateway-id "$gw_id"

    # Create a route table and a route to the Internet through the Gateway.
    rtb_id=$(aws ec2 create-route-table --vpc-id "$vpc_id" --query 'RouteTable.RouteTableId')
    aws ec2 create-route --route-table-id "$rtb_id" --destination-cidr-block 0.0.0.0/0 --gateway-id "$gw_id"

    # Create a subnet and associate the route table
    az=$(aws ec2 describe-availability-zones --query "AvailabilityZones[0].ZoneName")
    subnet_id=$(aws ec2 create-subnet --vpc-id "$vpc_id" --cidr-block 10.0.0.0/24 \
        --availability-zone "$az" --query 'Subnet.SubnetId')
    aws ec2 modify-subnet-attribute --subnet-id "$subnet_id" --map-public-ip-on-launch
    aws ec2 associate-route-table --subnet-id "$subnet_id" --route-table-id "$rtb_id"

    # Create a security group to allow TCP egress to the Internet.
    sg_id=$(aws ec2 create-security-group --group-name "${name}-sg" --vpc-id "$vpc_id" --query 'GroupId' \
        --description "Allow TCP egress and InfluxDB ingress")
    aws ec2 authorize-security-group-egress --group-id "$sg_id" --cidr 0.0.0.0/0 --protocol tcp --port 0-65535
    aws ec2 authorize-security-group-ingress --group-id "$sg_id" --cidr 10.0.0.0/24 --protocol tcp --port 8086

    # Allow SSH from the machine where this script is running, so we can provision instances.
    my_ip=$(curl -s https://checkip.amazonaws.com)
    aws ec2 authorize-security-group-ingress --group-id "$sg_id" --cidr "0.0.0.0/0" --protocol tcp --port 22

    # ----------------------------------------------------------------------------------------------------------------
    # CREATE INFLUXDB DATABASE

    # Create private endpoint for Amazon Timestream within the VPC
    aws ec2 create-vpc-endpoint --vpc-id "$vpc_id" --query 'VpcEndpoint.VpcEndpointId' \
        --service-name "com.amazonaws.$region.timestream-influxdb" --vpc-endpoint-type Interface \
        --subnet-ids "$subnet_id" --security-group-ids "$sg_id" --private-dns-enabled

    aws timestream-influxdb create-db-instance --name "${name}-db" \
        --vpc-subnet-ids "$subnet_id" --vpc-security-group-ids "$sg_id" --deployment-type "SINGLE_AZ" \
        --db-instance-type "db.influx.medium" --db-storage-type "InfluxIOIncludedT1" --allocated-storage 50 \
        --organization "metrics_corp" --bucket "metrics_corp_bucket" \
        --username "admin" --password "YourSecurePassword" --tags "Key=Name,Value=${name}-db"

    echo "Waiting for the timestream-influxdb instance to be available (this takes around 10 minutes)..."
    while ! aws timestream-influxdb list-db-instances \
        --query "items[?name=='${name}-db'].{Status: status}" | grep -q "AVAILABLE"; do
        sleep 60
    done
    db_endpoint=$(aws timestream-influxdb list-db-instances --query "items[?name=='${name}-db'].{Endpoint: endpoint}")

    # ----------------------------------------------------------------------------------------------------------------
    # CREATE INSTANCE

    ami_id=$(aws ec2 describe-images --owners 137112412989 --query "Images | sort_by(@, &CreationDate) | [-1].ImageId" \
        --filters "Name=name,Values=al2023-ami-2023*" "Name=architecture,Values=x86_64" \
                  "Name=virtualization-type,Values=hvm" "Name=root-device-type,Values=ebs" )

    aws ec2 create-key-pair --key-name "${name}-key" --query 'KeyMaterial' > key.pem
    chmod 400 key.pem

    sed "s/\$ENROLLMENT_TICKET/${enrollment_ticket}/g" run_ockam.sh > user_data1.sh
    sed "s/\$OCKAM_VERSION/${OCKAM_VERSION}/g" user_data1.sh > user_data2.sh
    sed "s/\$INFLUXDB_ADDRESS/${db_endpoint}/g" user_data2.sh > user_data.sh
    instance_id=$(aws ec2 run-instances --image-id "$ami_id" --instance-type c5n.large \
        --subnet-id "$subnet_id" --security-group-ids "$sg_id" \
        --key-name "${name}-key" --user-data file://user_data.sh  --query 'Instances[0].InstanceId')
    aws ec2 create-tags --resources "$instance_id" --tags "Key=Name,Value=${name}-ec2-instance"
    aws ec2 wait instance-running --instance-ids "$instance_id"
    ip=$(aws ec2 describe-instances --instance-ids "$instance_id" --query 'Reservations[0].Instances[0].PublicIpAddress')

    scp -o StrictHostKeyChecking=no -i ./key.pem "ec2-user@$ip:token.txt" ../datastream_corp/token.txt
}

cleanup() {

    # ----------------------------------------------------------------------------------------------------------------
    # DELETE INSTANCE

    rm -f user_data*.sh
    instance_ids=$(aws ec2 describe-instances --filters "Name=tag:Name,Values=${name}-ec2-instance" \
        --query "Reservations[*].Instances[*].InstanceId")
    for i in $instance_ids; do
        aws ec2 terminate-instances --instance-ids "$i"
        aws ec2 wait instance-terminated --instance-ids "$i"
    done

    if aws ec2 describe-key-pairs --key-names "${name}-key" &>/dev/null; then
        aws ec2 delete-key-pair --key-name "${name}-key"
    fi
    rm -f key.pem

    # ----------------------------------------------------------------------------------------------------------------
    # DELETE INFLUXDB DATABASE

    db_instance_ids=$(aws timestream-influxdb list-db-instances --query "items[?name=='${name}-db'].{id: id}")
    for i in $db_instance_ids; do
        aws timestream-influxdb delete-db-instance --identifier "$i"
        echo "Waiting for the timestream-influxdb instance be deleted (this takes around 10 minutes)..."
        while aws timestream-influxdb get-db-instance --identifier "$i" &>/dev/null; do sleep 60; done
    done

    # ----------------------------------------------------------------------------------------------------------------
    # DELETE NETWORK

    vpc_ids=$(aws ec2 describe-vpcs --query 'Vpcs[*].VpcId' --filters "Name=tag:Name,Values=${name}-vpc")

    for vpc_id in $vpc_ids; do
        vpc_endpoints=$(aws ec2 describe-vpc-endpoints --query 'VpcEndpoints[*].VpcEndpointId' \
            --filters "Name=vpc-id,Values=${vpc_id}")
        for i in $vpc_endpoints; do
            aws ec2 delete-vpc-endpoints --vpc-endpoint-ids "$i"
            while aws ec2 describe-vpc-endpoints --vpc-endpoint-ids "$i" &>/dev/null; do sleep 10; done
        done

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

check_if_timestream_is_available_in_the_configured_region() {
    configured_region="$1"
    supported_regions=$(aws ssm get-parameters-by-path \
        --path /aws/service/global-infrastructure/services/timestream/regions --query 'Parameters[].Value')

    if ! echo "$supported_regions" | grep -q "$configured_region"; then
        echo "Timestream is not available in the region '$configured_region' that you aws cli is configured to use.";
        echo "It is available in the following regions: $supported_regions"
        exit 1
    fi
}

export AWS_PAGER="";
export AWS_DEFAULT_OUTPUT="text";

user=""
command -v sha256sum &>/dev/null && user=$(aws sts get-caller-identity | sha256sum | cut -c 1-20)
command -v shasum &>/dev/null && user=$(aws sts get-caller-identity | shasum -a 256 | cut -c 1-20)
export name="ockam-ex-ts-m-$user" #Limit to 40 chars: db name constraint

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then cleanup; else run "$1"; fi
