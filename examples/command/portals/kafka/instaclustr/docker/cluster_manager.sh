#!/bin/bash

# Function to create a Kafka cluster
create_cluster() {
  local response
  local create_cluster_url="https://api.instaclustr.com/cluster-management/v2/resources/applications/kafka/clusters/v2"
  response=$(curl -s -w "%{http_code}" -X POST -u "${INSTACLUSTR_USER_NAME}:${INSTACLUSTR_API_KEY}" "${create_cluster_url}" \
    -H 'Content-Type: application/json' \
    -d '{
      "allowDeleteTopics": true,
      "autoCreateTopics": true,
      "clientToClusterEncryption": false,
      "dataCentres": [
        {
          "cloudProvider": "AWS_VPC",
          "name": "AWS_VPC_US_EAST_1",
          "network": "10.0.0.0/16",
          "nodeSize": "KFK-DEV-t4g.small-5",
          "numberOfNodes": 3,
          "region": "US_EAST_1"
        }
      ],
      "dedicatedZookeeper": [],
      "defaultNumberOfPartitions": 3,
      "defaultReplicationFactor": 3,
      "kafkaVersion": "3.6.1",
      "name": "DemoKafkaCluster",
      "pciComplianceMode": false,
      "privateNetworkCluster": false,
      "slaTier": "NON_PRODUCTION"
    }')
  echo "${response}"
}

# Function to wait for cluster to be fully operational
wait_for_cluster() {
  local cluster_id=$1
  local status
  local response

  while true; do
    local url="https://api.instaclustr.com/cluster-management/v2/resources/applications/kafka/clusters/v2/${cluster_id}"
    response=$(curl -s -X GET -H "Content-Type: application/json" -u "${INSTACLUSTR_USER_NAME}:${INSTACLUSTR_API_KEY}" "${url}")
    status=$(echo "${response}" | jq -r '.status')

    case "${status}" in
      RUNNING)
        echo "Cluster is now running. Waiting for 60 seconds to be fully available"
        sleep 60
        break
        ;;
      ERROR)
        echo "Cluster creation failed with status: ${status}"
        echo "Error details: $(echo "${response}" | jq -r '.errorMessage')"
        exit 1
        ;;
      DEFERRED)
        echo "Cluster creation has been deferred. Exiting..."
        exit 1
        ;;
      *)
        echo "Current status: ${status}. Waiting for cluster to be in RUNNING status..."
        sleep 60
        ;;
    esac
  done
}

# Function to retrieve cluster details
get_cluster_details() {
  local cluster_id=$1
  local response
  response=$(curl -s -X GET -H "Content-Type: application/json" -u "${INSTACLUSTR_USER_NAME}:${INSTACLUSTR_API_KEY}" \
    "https://api.instaclustr.com/cluster-management/v2/resources/applications/kafka/clusters/v2/${cluster_id}")
  echo "${response}"
}

# Function to add a firewall rule
add_firewall_rule() {
    local cluster_id=$1
    local my_ip=$(curl -s https://checkip.amazonaws.com)
    local response

    response=$(curl -s -X POST -H "Content-Type: application/json" -u "${INSTACLUSTR_USER_NAME}:${INSTACLUSTR_API_KEY}" \
      "https://api.instaclustr.com/cluster-management/v2/resources/network-firewall-rules/v2" \
      -d '{
          "clusterId": "'"${cluster_id}"'",
          "network": "'"${my_ip}/32"'",
          "type":"KAFKA"
        }')
    #echo "${response}"
    local status=$(echo "${response}" | jq -r '.status')
    if [[ "${status}" == "GENESIS" ]]; then
        echo "Firewall rule applied successfully"
    else
        echo "Firewall rule application failed"
    fi
}

# Function to create a user for the Kafka cluster
create_user() {
  local cluster_id=$1
  local response
  response=$(curl -s -X POST -u "${INSTACLUSTR_USER_NAME}:${INSTACLUSTR_API_KEY}" \
    "https://api.instaclustr.com/cluster-management/v2/resources/applications/kafka/users/v2" \
    -H 'Content-Type: application/json' \
    -d '{
      "clusterId": "'"${cluster_id}"'",
      "initialPermissions": "standard",
      "options": {
        "overrideExistingUser": false,
        "saslScramMechanism": "SCRAM-SHA-256"
      },
      "password": "myPassword1.",
      "username": "myKafkaUser"
    }')
  # echo "${response}"
  if [[ $(echo "${response}" | jq -r '.id') != "null" && $(echo "${response}" | jq -r '.username') == "myKafkaUser" ]]; then
      echo "User created successfully"
  else
      echo "User creation failed"
  fi
}

# Function to delete the Kafka cluster
delete_cluster() {
  local cluster_id=$1
  local response
  echo "Deleting Kafka cluster"
  response=$(curl -s -X DELETE -u "${INSTACLUSTR_USER_NAME}:${INSTACLUSTR_API_KEY}" \
    "https://api.instaclustr.com/cluster-management/v2/resources/applications/kafka/clusters/v2/${cluster_id}")
  echo "${response}"
}

# Main function
main() {
  if [[ "$1" == "cleanup" ]]; then
    local cluster_id
    cluster_id=$(cat cluster_id.txt)
    delete_cluster "${cluster_id}"
    rm cluster_id.txt
    unset INSTACLUSTR_USER_NAME
    unset INSTACLUSTR_API_KEY
  else
    local response=$(create_cluster)
    local http_code=$(echo "${response}" | tail -c 4)
    local response_body=$(echo "${response}" | sed '$ s/...$//')

    if [[ $http_code -eq 401 ]]; then
      echo "Unauthorized: Please check your username or API key"
      exit 1
    fi

    local cluster_id=$(echo "${response_body}" | jq -r '.id')
    if [ -z "$cluster_id" ]; then
      echo "Error: Cluster creation failed"
      exit 1
    fi
    echo "Creating a trial cluster. Cluster setup usually take around 5-10 minutes"
    echo "Cluster ID: ${cluster_id}"
    echo "${cluster_id}" > cluster_id.txt
    wait_for_cluster "${cluster_id}"
    create_user "${cluster_id}"
    add_firewall_rule "${cluster_id}"
    local cluster_details=$(get_cluster_details "${cluster_id}")
    local bootstrapserver=$(echo "${cluster_details}" | jq -r '.dataCentres[].nodes[-2].publicAddress')
    echo "BOOTSTRAP_SERVER:$bootstrapserver"
  fi
}

# Run the main script logic
main "$@"
