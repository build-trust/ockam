#!/usr/bin/env bash
set -ex
run() {
    enrollment_ticket="$1"
    check_model_availability

    # ----------------------------------------------------------------------------------------------------------------
    # CREATE NETWORK

    # Create a new VPC and tag it.
    vpc_id=$(aws ec2 create-vpc --cidr-block 10.0.0.0/16 --query 'Vpc.VpcId')
    aws ec2 create-tags --resources "$vpc_id" --tags "Key=Name,Value=${name}-vpc"
    aws ec2 modify-vpc-attribute --vpc-id "$vpc_id" --enable-dns-support '{"Value":true}'
    aws ec2 modify-vpc-attribute --vpc-id "$vpc_id" --enable-dns-hostnames '{"Value":true}'

    # Create and Internet Gateway and attach it to the VPC.
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

    # Create a security group to allow:
    #   - TCP egress to the Internet
    #   - SSH ingress only from withing our two subnets.
    sg_id=$(aws ec2 create-security-group --group-name $security_group_name --vpc-id "$vpc_id" --query 'GroupId' \
        --description "Allow TCP egress and SSH ingress")
    aws ec2 authorize-security-group-egress --group-id "$sg_id" --cidr 0.0.0.0/0 --protocol tcp --port 0-65535
    aws ec2 authorize-security-group-ingress --group-id "$sg_id" --cidr 0.0.0.0/0 --protocol tcp --port 22

    region=$(aws ec2 describe-availability-zones --query 'AvailabilityZones[0].[RegionName]')
    vpce_id=$(aws ec2 create-vpc-endpoint --vpc-id "$vpc_id" --query 'VpcEndpoint.VpcEndpointId' \
        --service-name "com.amazonaws.$region.bedrock" --vpc-endpoint-type Interface \
        --subnet-ids "$subnet_id" --security-group-ids "$sg_id" --private-dns-enabled )
    vpce_dns_name="bedrock.$region.amazonaws.com"

    # ----------------------------------------------------------------------------------------------------------------
    # CREATE INSTANCE

    ami_id=$(aws ec2 describe-images --owners 137112412989 --query "Images | sort_by(@, &CreationDate) | [-1].ImageId" \
        --filters "Name=name,Values=al2023-ami-2023*" "Name=architecture,Values=x86_64" \
                  "Name=virtualization-type,Values=hvm" "Name=root-device-type,Values=ebs" )

    aws ec2 create-key-pair --key-name "$key_name" --query 'KeyMaterial' > key.pem
    chmod 400 key.pem

    sed "s/\$ENROLLMENT_TICKET/${enrollment_ticket}/g" run_ockam.sh > user_data1.sh
    sed "s/\$OCKAM_VERSION/${OCKAM_VERSION}/g" user_data1.sh > user_data.sh
    instance_id=$(aws ec2 run-instances --image-id "$ami_id" --instance-type c5n.large \
        --subnet-id "$subnet_id" --security-group-ids "$sg_id" \
        --key-name "$key_name" --user-data file://user_data.sh --query 'Instances[0].InstanceId')
    aws ec2 create-tags --resources "$instance_id" --tags "Key=Name,Value=${name}-ec2-instance"

    aws iam create-role --role-name $ai_role --assume-role-policy-document file://trust-policy.json
    policy_arn=$(aws iam create-policy --policy-name $ai_policy --policy-document file://policy.json --query "Policy.Arn")
    account_id="$(aws sts get-caller-identity --query Account)"
    aws iam attach-role-policy --role-name "$ai_role" --policy-arn $policy_arn

    aws iam create-instance-profile --instance-profile-name $ai_profile
    aws iam add-role-to-instance-profile --instance-profile-name $ai_profile --role-name $ai_role
    # we need to wait a bit for the next operation to succeed
    sleep 5
    aws ec2 associate-iam-instance-profile --instance-id $instance_id --iam-instance-profile Name=$ai_profile

    aws ec2 wait instance-running --instance-ids "$instance_id"
    ip=$(aws ec2 describe-instances --instance-ids "$instance_id" --query 'Reservations[0].Instances[0].PublicIpAddress')
    rm -f user_data.sh

    until scp -o StrictHostKeyChecking=no -i ./key.pem ./api.mjs "ec2-user@$ip:api.mjs"; do sleep 10; done
    until scp -o StrictHostKeyChecking=no -i ./key.pem ./constants.mjs "ec2-user@$ip:constants.mjs"; do sleep 10; done
    ssh -o StrictHostKeyChecking=no -i ./key.pem "ec2-user@$ip" \
        'bash -s' << 'EOS'
            sudo yum update -y && sudo yum install nodejs -y
            npm install express @aws-sdk/client-bedrock-runtime

            echo "Start the AI API"
            nohup node api.mjs &>output.log &
            echo "AI API started"
EOS
}

check_model_availability() {
    supported_regions=$(aws ssm get-parameters-by-path \
        --path /aws/service/global-infrastructure/services/bedrock/regions --output json | \
        jq -r '.Parameters[].Value')

    configured_region=$(get_configured_region)

    # Check if the configured region is in the list of supported regions
    if echo "${supported_regions}" | grep -q "${configured_region}"; then
        echo "Amazon Bedrock is available in the ${configured_region} region. Proceeding..."
    else
        echo "Amazon Bedrock is not available in ${configured_region} region "
        echo "Please use one of the supported regions for this example to run:"
        echo "${supported_regions}"
        exit 1
    fi

    # check if the model access has been granted
    npm install @aws-sdk/client-bedrock-runtime
    node ./check-model-availability.mjs
    if [ $? -eq 0 ]; then
        echo "The amazon.titan-text-lite-v1 model is accessible"
    else
        echo "The amazon.titan-text-lite-v1 model is not accessible."
        echo "Please go to https://${configured_region}.console.aws.amazon.com/bedrock/home?region=${configured_region}#/modelaccess to request an access to that model."
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
    rm -f user_data*.sh

    instance_ids=$(aws ec2 describe-instances --filters "Name=tag:Name,Values=${name}-ec2-instance" \
        --query "Reservations[].Instances[?State.Name!='terminated'].InstanceId[]")
    for instance in $instance_ids; do
        profile_associations="$(aws ec2 describe-iam-instance-profile-associations \
            --filter "Name=instance-id,Values='$instance'" \
            --query 'IamInstanceProfileAssociations[].AssociationId')"

        for profile_association in $profile_associations; do
            aws ec2 disassociate-iam-instance-profile --association-id $profile_association
        done
    done

    policies=$(aws iam list-policies --query "Policies[?contains(PolicyName, '$ai_policy')].Arn")
    for policy_arn in $policies; do
        aws iam detach-role-policy --role-name $ai_role --policy-arn $policy_arn
        aws iam delete-policy --policy-arn $policy_arn
    done

    aws iam remove-role-from-instance-profile --instance-profile-name $ai_profile --role-name $ai_role || true
    aws iam delete-role --role-name $ai_role || true
    aws iam delete-instance-profile --instance-profile-name $ai_profile || true

    for instance in $instance_ids; do
        aws ec2 terminate-instances --instance-ids "$instance"
        aws ec2 wait instance-terminated --instance-ids "$instance"
    done

    if aws ec2 describe-key-pairs --key-names "$key_name" &>/dev/null; then
        aws ec2 delete-key-pair --key-name "$key_name"
    fi
    rm -f key.pem

    # ----------------------------------------------------------------------------------------------------------------
    # DELETE NETWORK


    vpc_ids=$(aws ec2 describe-vpcs \
        --filters "Name=tag:Name,Values=${name}-vpc" \
        --query 'Vpcs[*].VpcId')

    for vpc_id in $vpc_ids; do
        vpc_endpoints=$(aws ec2 describe-vpc-endpoints \
            --filters "Name=vpc-id,Values=${vpc_id}" \
            --query 'VpcEndpoints[*].VpcEndpointId')
        for i in $vpc_endpoints; do
            aws ec2 delete-vpc-endpoints --vpc-endpoint-ids "$i"
            while aws ec2 describe-vpc-endpoints --vpc-endpoint-ids "$i" &>/dev/null; do sleep 10; done
        done

        internet_gateways=$(aws ec2 describe-internet-gateways \
            --query "InternetGateways[*].InternetGatewayId" \
            --filters Name=attachment.vpc-id,Values="$vpc_id")
        for i in $internet_gateways; do
            aws ec2 detach-internet-gateway --internet-gateway-id "$i" --vpc-id "$vpc_id"
            aws ec2 delete-internet-gateway --internet-gateway-id "$i"
        done

        subnet_ids=$(aws ec2 describe-subnets \
            --query "Subnets[*].SubnetId" \
            --filters Name=vpc-id,Values="$vpc_id")
        for i in $subnet_ids; do aws ec2 delete-subnet --subnet-id "$i"; done

        route_table_associations=$(aws ec2 describe-route-tables \
            --filters Name=vpc-id,Values="$vpc_id" \
            --query 'RouteTables[?length(Associations[?Main!=`true`]) > `0`].Associations[].RouteTableAssociationId')
        for i in $route_table_associations; do aws ec2 disassociate-route-table --association-id "$i" || true; done

        route_tables=$(aws ec2 describe-route-tables \
            --filters Name=vpc-id,Values="$vpc_id" \
            --query 'RouteTables[?length(Associations[?Main!=`true`]) > `0` || length(Associations) == `0`].RouteTableId')
        for i in $route_tables; do aws ec2 delete-route-table --route-table-id "$i" || true; done

        security_groups=$(aws ec2 describe-security-groups \
            --filters Name=vpc-id,Values="$vpc_id" \
            --query "SecurityGroups[?GroupName=='$security_group_name'].GroupId")
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
export name="ockam-ex-ai-bedrock-ai-corp-$user"

export ai_profile="${name}-ai-profile"
export ai_role="${name}-ai-role"
export ai_policy="${name}-ai-policy"
export security_group_name="${name}-sg"
export key_name="${name}-key"

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then cleanup; else run "$1"; fi
