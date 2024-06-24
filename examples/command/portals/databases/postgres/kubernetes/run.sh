#!/usr/bin/env bash
set -e

# This script, `./run.sh ...` is invoked on a developer’s work machine.
#
# This hands-on example uses Ockam to create an end-to-end encrypted portal to postgres. We
# connect a nodejs app in one private kubernetes cluster with a postgres database in another
# private kubernetes cluster.
#
# The example uses docker and kind to create these kubernetes clusters.
#
# You can read a detailed walkthrough of this example at:
# https://docs.ockam.io/portals/databases/postgres/kubernetes

run() {
    # Run `ockam enroll`.
    #
    # The enroll command creates a new vault and generates a cryptographic identity with
    # private keys stored in that vault. It then guides you to sign in to Ockam Orchestrator.
    #
    # If this is your first time signing in, the Orchestrator creates a new dedicated project
    # for you. A project offers two services: a membership authority and a relay service.
    #
    # The enroll command then asks this project’s membership authority to sign and issue
    # a credential that attests that your identifier is a member of this project. Since your
    # account in Orchestrator is the creator and hence first administrator on this new project,
    # the membership authority issues this credential. The enroll command stores the
    # credential for later use and exits.
    ockam enroll

    # Create an enrollment ticket to enroll the identity used by an ockam node that will run
    # adjacent to the postgres server in bank_corp's network.
    #
    # The identity that enrolls with the generated ticket will be given a project membership
    # credential in which the project membership authority will cryptographically attest to the
    # specified attributes - postgres-outlet=true.
    #
    # The identity will also allowed to create a relay in the project at the address `postgres`.
    bank_corp_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m \
        --attribute "postgres-outlet=true" --relay postgres)

    # Create an enrollment ticket to enroll the identity used by an ockam node that will run
    # adjacent to the postgres client app in analysis_corp's network.
    #
    # The identity that enrolls with the generated ticket will be given a project membership
    # credential in which the project membership authority will cryptographically attest to the
    # specified attributes - postgres-inlet=true.
    analysis_corp_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m \
        --attribute "postgres-inlet=true")

    # Create bank_corp's kubernetes cluster.
    #
    # Read bank_corp/pod.yml to understand the parts that are provisioned
    # in bank_corp's kubernetes cluster.
    pushd bank_corp
        # Create a kubernetes cluster called bank-corp
        kind create cluster --name bank-corp
        sleep 30

        # Build the ockam_node_bank_corp:v1 docker image and load it into the bank-corp kubernetes cluster.
        build_and_load_docker_image ockam_node_bank_corp:v1 ../ockam.dockerfile . bank-corp

        # Pass the above enrollment ticket as a kubenetes secret.
        kubectl create secret generic ockam-node-enrollment-ticket \
            "--from-literal=ticket=$bank_corp_ticket" --context kind-bank-corp

        # Apply the kubernetes manifest at bank_corp/pod.yaml
        kubectl apply -f pod.yml --context kind-bank-corp
    popd

    # Create analysis_corp's kubernetes cluster.
    #
    # Read analysis_corp/pod.yml to understand the parts that are provisioned
    # in bank_corp's kubernetes cluster.
    pushd analysis_corp
        # Create a kubernetes cluster called analysis-corp
        kind create cluster --name analysis-corp
        sleep 30

        # Build ockam_node_analysis_corp:v1 and app:v1 docker images
        # and load them into the analysis-corp kubernetes cluster.
        build_and_load_docker_image ockam_node_analysis_corp:v1 ../ockam.dockerfile . analysis-corp
        build_and_load_docker_image app:v1 app.dockerfile . analysis-corp

        # Pass the above enrollment ticket as a kubenetes secret.
        kubectl create secret generic ockam-node-enrollment-ticket \
            "--from-literal=ticket=$analysis_corp_ticket" --context kind-analysis-corp

        # Apply the kubernetes manifest at analysis_corp/pod.yaml
        kubectl apply -f pod.yml --context kind-analysis-corp
    popd

    until kubectl logs --follow app-ockam-pod -c app --context kind-analysis-corp 2> /dev/null; do sleep 2; done
}

# Build a docker image and load it into a kind kubernetes cluster.
build_and_load_docker_image() {
    tag="$1"; dockerfile="$2"; context="$3"; cluster="$4"

    if [[ -z $OCKAM_VERSION ]]; then
        export OCKAM_VERSION="latest"
    fi

    # Use --load option only if docker buildx is available.
    if docker buildx ls &>/dev/null; then
        docker build --build-arg OCKAM_VERSION="$OCKAM_VERSION" --load --file "$dockerfile" --tag "$tag" "$context"
    else
        docker build --build-arg OCKAM_VERSION="$OCKAM_VERSION" --file "$dockerfile" --tag "$tag" "$context"
    fi

    kind load docker-image "$tag" --name "$cluster"
}

# Cleanup after the example - `./run.sh cleanup`
# Remove all clusters, containers, and images created by this example.
cleanup() {
    pushd bank_corp; kind delete cluster --name bank-corp; popd
    pushd analysis_corp; kind delete cluster --name analysis-corp; popd
    docker rmi ockam_node_bank_corp:v1 ockam_node_analysis_corp:v1 app:v1 2> /dev/null
}

# Check if Ockam Command is already installed and available in path.
# If it's not, then install it.
if ! type ockam &>/dev/null && ! [[ "$1" = "cleanup" ]]; then
    curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
    source "$HOME/.ockam/env"
fi

# Check that tools we we need installed.
for c in docker kind kubectl curl; do
    if ! type "$c" &>/dev/null; then echo "ERROR: Please install: $c" && exit 1; fi
done

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then cleanup; else run; fi
