set -e

if [[ -n $SET_AWS_REGION ]]; then
    export AWS_DEFAULT_REGION="$SET_AWS_REGION"
fi

if [[ -n $ENABLE_SSH ]]; then
    set +x
fi

common_create(){
    enrollment_ticket="$1"
    file_name="$2"
    zone_id="$3"

    # ----------------------------------------------------------------------------------------------------------------
    # CREATE NETWORK

    # Create a new VPC and tag it.
    vpc_id=$(aws ec2 create-vpc --cidr-block 10.0.0.0/16 --query 'Vpc.VpcId')
    aws ec2 create-tags --resources "$vpc_id" --tags "Key=Name,Value=${name}-vpc" >/dev/null

    # Create an Internet Gateway and attach it to the VPC.
    gw_id=$(aws ec2 create-internet-gateway --query 'InternetGateway.InternetGatewayId')
    aws ec2 attach-internet-gateway --vpc-id "$vpc_id" --internet-gateway-id "$gw_id" >/dev/null

    # Create a route table and a route to the Internet through the Gateway.
    rtb_id=$(aws ec2 create-route-table --vpc-id "$vpc_id" --query 'RouteTable.RouteTableId')
    aws ec2 create-route --route-table-id "$rtb_id" --destination-cidr-block 0.0.0.0/0 --gateway-id "$gw_id" >/dev/null

    # Create a subnet and associate the route table
    az=$(aws ec2 describe-availability-zones --query "AvailabilityZones[${zone_id}].ZoneName")
    subnet_id=$(aws ec2 create-subnet --vpc-id "$vpc_id" --cidr-block 10.0.0.0/25 \
        --availability-zone "$az" --query 'Subnet.SubnetId')
    aws ec2 modify-subnet-attribute --subnet-id "$subnet_id" --map-public-ip-on-launch >/dev/null
    aws ec2 associate-route-table --subnet-id "$subnet_id" --route-table-id "$rtb_id" >/dev/null

    # Create a security group to allow:
    #   - TCP egress to the Internet
    #   - SSH ingress from the Internet if ENABLE_SSH is set
    sg_id=$(aws ec2 create-security-group --group-name "${name}-sg" --vpc-id "$vpc_id" --query 'GroupId' \
        --description "Allow TCP egress and Postgres ingress")
    aws ec2 authorize-security-group-egress --group-id "$sg_id" --cidr 0.0.0.0/0 --protocol tcp --port 0-65535 >/dev/null
    aws ec2 authorize-security-group-egress --group-id "$sg_id" --cidr 0.0.0.0/0 --protocol udp --port 0-65535 >/dev/null

    if [[ -n $ENABLE_SSH ]]; then
        aws ec2 authorize-security-group-ingress --group-id "$sg_id" --cidr 0.0.0.0/0 --protocol tcp --port 22 >/dev/null

        aws ec2 create-key-pair --key-name "${name}-key" --query 'KeyMaterial' > key.pem
        chmod 400 key.pem
    fi

    # ----------------------------------------------------------------------------------------------------------------
    # CREATE INSTANCE

    ami_id=$(aws ec2 describe-images --owners 137112412989 --query "Images | sort_by(@, &CreationDate) | [-1].ImageId" \
        --filters "Name=name,Values=al2023-ami-2023*" "Name=architecture,Values=x86_64" \
                    "Name=virtualization-type,Values=hvm" "Name=root-device-type,Values=ebs" )

    sed "s/\$ENROLLMENT_TICKET/${enrollment_ticket}/g" ${file_name} > user_data1.sh
    sed "s/\$OCKAM_VERSION/${OCKAM_VERSION}/g" user_data1.sh > user_data.sh

    if [[ -n $ENABLE_SSH ]]; then
        instance_id=$(aws ec2 run-instances --image-id "$ami_id" --instance-type c5n.large \
            --subnet-id "$subnet_id" --security-group-ids "$sg_id" \
            --key-name "${name}-key" --user-data file://user_data.sh --query 'Instances[0].InstanceId')
    else
        instance_id=$(aws ec2 run-instances --image-id "$ami_id" --instance-type c5n.large \
            --subnet-id "$subnet_id" --security-group-ids "$sg_id" \
            --user-data file://user_data.sh --query 'Instances[0].InstanceId')
    fi

    aws ec2 create-tags --resources "$instance_id" --tags "Key=Name,Value=${name}-ec2-instance" >/dev/null
    aws ec2 wait instance-running --instance-ids "$instance_id" >/dev/null

    if [[ -n $ENABLE_SSH ]]; then
        ip=$(aws ec2 describe-instances --instance-ids "$instance_id" --query 'Reservations[0].Instances[0].PublicIpAddress')
        echo "$ip"
    fi
}

common_cleanup() {
    # ----------------------------------------------------------------------------------------------------------------
    # DELETE INSTANCE

    rm -rf user_data*.sh
    instance_ids=$(aws ec2 describe-instances --filters "Name=tag:Name,Values=${name}-ec2-instance" \
        --query "Reservations[*].Instances[*].InstanceId")
    for i in $instance_ids; do
        aws ec2 terminate-instances --instance-ids "$i" >/dev/null
        aws ec2 wait instance-terminated --instance-ids "$i" >/dev/null
    done

    if [[ -n $ENABLE_SSH ]]; then
        if aws ec2 describe-key-pairs --key-names "${name}-key" >/dev/null; then
            aws ec2 delete-key-pair --key-name "${name}-key" >/dev/null
        fi
        rm -f key.pem
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