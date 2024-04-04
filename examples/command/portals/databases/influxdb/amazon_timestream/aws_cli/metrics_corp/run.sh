#!/usr/bin/env bash
set -ex

run() {
    enrollment_ticket="$1"
    check_timestream_availability
    # ----------------------------------------------------------------------------------------------------------------
    # CREATE NETWORK

    # Create a new VPC and tag it.
    vpc_id=$(aws ec2 create-vpc --cidr-block 10.0.0.0/16 --query 'Vpc.VpcId')
    aws ec2 create-tags --resources "$vpc_id" --tags "Key=Name,Value=${name}-vpc"

    # Create and Internet Gateway and attach it to the VPC.
    gw_id=$(aws ec2 create-internet-gateway --query 'InternetGateway.InternetGatewayId')
    aws ec2 attach-internet-gateway --vpc-id "$vpc_id" --internet-gateway-id "$gw_id"

    # Create a route table and a route to the Internet through the Gateway.
    rtb_id=$(aws ec2 create-route-table --vpc-id "$vpc_id" --query 'RouteTable.RouteTableId')
    aws ec2 create-route --route-table-id "$rtb_id" --destination-cidr-block 0.0.0.0/0 --gateway-id "$gw_id"

    az1=$(aws ec2 describe-availability-zones --query "AvailabilityZones[0].ZoneName")
    subnet1_id=$(aws ec2 create-subnet --vpc-id "$vpc_id" --cidr-block 10.0.0.0/25 \
        --availability-zone "$az1" --query 'Subnet.SubnetId')
    aws ec2 modify-subnet-attribute --subnet-id "$subnet1_id" --map-public-ip-on-launch
    aws ec2 associate-route-table --subnet-id "$subnet1_id" --route-table-id "$rtb_id"

    az2=$(aws ec2 describe-availability-zones --query "AvailabilityZones[1].ZoneName")
    subnet2_id=$(aws ec2 create-subnet --vpc-id "$vpc_id" --cidr-block 10.0.0.128/25 \
        --availability-zone "$az2" --query 'Subnet.SubnetId')
    aws ec2 modify-subnet-attribute --subnet-id "$subnet2_id" --map-public-ip-on-launch
    aws ec2 associate-route-table --subnet-id "$subnet2_id" --route-table-id "$rtb_id"

    # Create a security group to allow:
    #   - TCP egress to the Internet
    #   - InfluxDB ingress from within two subnets.
    #   - InfluxDB ingress from within local machine running the script. To create API Key
    my_ip=$(curl -s https://checkip.amazonaws.com)
    sg_id=$(aws ec2 create-security-group --group-name "${name}-sg" --vpc-id "$vpc_id" --query 'GroupId' \
        --description "Allow TCP egress and InfluxDB ingress")
    aws ec2 authorize-security-group-egress --group-id "$sg_id" --cidr 0.0.0.0/0 --protocol tcp --port 0-65535
    aws ec2 authorize-security-group-ingress --group-id "$sg_id" --cidr 10.0.0.0/24 --protocol tcp --port 8086
    aws ec2 authorize-security-group-ingress --group-id "$sg_id" --cidr "${my_ip}/32" --protocol tcp --port 8086
    aws ec2 authorize-security-group-ingress --group-id "$sg_id" --cidr "${my_ip}/32" --protocol tcp --port 22
    # ----------------------------------------------------------------------------------------------------------------
    # CREATE INFLUXDB DATABASE

    aws timestream-influxdb create-db-instance --name "${name}-db"  --vpc-subnet-ids "$subnet1_id" "$subnet2_id" \
    --vpc-security-group-ids "$sg_id" \
    --publicly-accessible \
    --deployment-type "SINGLE_AZ" \
    --username "admin" \
    --password "YourSecurePassword" \
    --organization "ockam_org" \
    --bucket "ockam_demo_bucket" \
    --db-instance-type "db.influx.medium" \
    --db-storage-type "InfluxIOIncludedT1" \
    --allocated-storage 50 \
    --tags "Key=Name,Value=${name}-db"

    while true; do
        # Query the status of the specified instance
        CURRENT_STATUS=$(aws timestream-influxdb list-db-instances --query "items[?name=='${name}-db'].{Status: status}" --output text)
        echo "Current Status: $CURRENT_STATUS"

        if [[ "$CURRENT_STATUS" == "AVAILABLE" ]]; then
            echo "${name}-db is now ready"
            break
        else
            echo "Waiting for ${name}-db to become Available..."
        fi

        sleep 60
    done

    db_endpoint=$(aws timestream-influxdb list-db-instances --query "items[?name=='${name}-db'].{Endpoint: endpoint}" --output text)

    sleep 20
    ./run_influx_auth.sh $db_endpoint
    # ----------------------------------------------------------------------------------------------------------------
    # CREATE INSTANCE

    ami_id=$(aws ec2 describe-images --owners 137112412989 --query "Images | sort_by(@, &CreationDate) | [-1].ImageId" \
        --filters "Name=name,Values=al2023-ami-2023*" "Name=architecture,Values=x86_64" \
                  "Name=virtualization-type,Values=hvm" "Name=root-device-type,Values=ebs" )
    aws ec2 create-key-pair --key-name "${name}-key" --query 'KeyMaterial' > key.pem

    sed "s/\$ENROLLMENT_TICKET/${enrollment_ticket}/g" run_ockam.sh > user_data1.sh
    sed "s/\$INFLUXDB_ADDRESS/${db_endpoint}/g" user_data1.sh > user_data.sh
    instance_id=$(aws ec2 run-instances --image-id "$ami_id" --instance-type c5n.large \
        --subnet-id "$subnet1_id" --security-group-ids "$sg_id" \
        --user-data file://user_data.sh  \
        --query 'Instances[0].InstanceId')
    aws ec2 create-tags --resources "$instance_id" --tags "Key=Name,Value=${name}-ec2-instance"
    aws ec2 wait instance-running --instance-ids "$instance_id"
    public_ip=$(aws ec2 describe-instances --instance-ids "$instance_id" --query 'Reservations[0].Instances[0].PublicIpAddress' --output text)
    aws ec2 authorize-security-group-ingress --group-id "$sg_id" --cidr "${public_ip}/32" --protocol tcp --port 8086
    rm -f user_data.sh user_data1.sh
}

check_timestream_availability() {
    supported_regions=$(aws ssm get-parameters-by-path \
        --path /aws/service/global-infrastructure/services/timestream/regions --output json | \
        jq -r '.Parameters[].Value')

    configured_region=$(get_configured_region)

    # Check if the configured region is in the list of supported regions
    if echo "${supported_regions}" | grep -q "${configured_region}"; then
        echo "Amazon Timestream is available in the ${configured_region} region. Proceeding..."
    else
        echo "Amazon Timestream is not available in ${configured_region} region "
        echo "Please use one of the supported regions for this example to run:"
        echo "${supported_regions}"
        exit 1
    fi
}

get_configured_region() {
    region=$(aws ec2 describe-availability-zones --query 'AvailabilityZones[0].[RegionName]' --output text)

    # Check if we have a region value
    if [[ -z "${region}" ]]; then
        echo "No AWS region is configured or set in environment variables."
        exit 1
    fi
    echo "${region}"
}

cleanup() {
    # ----------------------------------------------------------------------------------------------------------------
    # DELETE INSTANCE

    rm -f user_data.sh user_data1.sh
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
    # DELETE DATABASE
    db_identifier=$(aws timestream-influxdb list-db-instances --query "items[?name=='${name}-db'].{id: id}" --output text)

    if [[ -n $db_identifier ]]; then
        aws timestream-influxdb delete-db-instance --identifier $db_identifier
        while true; do
            result=$(aws timestream-influxdb list-db-instances --query "items[?name=='${name}-db'].{id: id}" --output json)

        if [[ "$result" == "[]" ]]; then
                echo "DB instance deleted."
                break
            else
                echo "Waiting for DB instance to be deleted..."
                sleep 60
        fi
        done
    else
        echo "DB instance does not exist or has already been deleted."
    fi
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
export name="ockam-metrics-$user" #Limit to 40 chars: db name constraint

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then cleanup; else run "$1"; fi
