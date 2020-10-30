#!/usr/bin/env sh

if [[ -z "$1" ]]; then
    echo "Ockam Demo: InfluxDB Add-on"
    echo "" 
    echo "USAGE"
    echo "" 
    echo "$ ./tools/docker/demo/influxdb.sh [COMPONENT] [ARGS]"
    echo "" 
    echo "COMPONENTS"
    echo ""
    echo "  influxdb-ockamd"
    echo "      starts the responder (sink) end, containing \`influxdb\` and \`ockamd\`, with configuration to send \`influxdb\` measurement data via \`ockamd\` over HTTP."
    echo "" 
    echo "  telegraf-ockamd [RESPONDER-PUBLIC-KEY]"
    echo "      starts the initiator (source) end, containing \`telegraf\` and \`ockamd\`, with configuration to start \`telegraf\` to use \`ockamd\` as an \"execd\" output plugin."
    echo ""
    echo "  telegraf-write"
    echo "      sends a random 'temperature' measurement to \`telegraf\` agent, which is encrypted by \`ockamd\` and written to \`influxdb\`."
    echo ""
    echo "  influxdb-query"
    echo "      executes a 'SELECT * FROM temperature' query within the 'ockam_demo' database in \`influxdb\`."
    echo ""
    echo "  kill-all"
    echo "      kills and removes all demo containers."
    echo ""
fi

case $1 in 
    influxdb-ockamd)
        # start the responder (sink) end, containing `influxdb` and `ockamd`, with configuration to 
        # send `influxdb` measurement data via `ockamd` over HTTP.
        docker run -d --network="host" --name="influxdb-ockamd" ockam/influxdb-ockamd:0.1.0 \
            --role=responder \
            --local-socket=127.0.0.1:52440 \
            --addon=influxdb,ockam_demo,http://localhost:8086 > /dev/null
        docker logs influxdb-ockamd
        docker exec influxdb-ockamd influx -execute 'CREATE DATABASE ockam_demo'

        echo ""
        echo "NOTE: copy the hex value printed above in the line prefixed with 'Responder public key:', and use it as the first argument in the \`telegraf-ockamd\` component command."
        ;;

    telegraf-ockamd)
        # start the initiator (source) end, containing `telegraf` and `ockamd`, with configuration
        # to start `telegraf` to use `ockamd` as an "execd" output plugin:
        if [[ -z "$2" ]]; then
            echo "ERROR: You must provide the responder public key returned from the previous script."
            exit 1
        fi

        docker run -d --network="host" --name="telegraf-ockamd" \
            --env OCKAMD_RESPONDER_PUBLIC_KEY=$2 \
            --env OCKAMD_LOCAL_SOCKET=127.0.0.1:52441 \
            --env OCKAMD_ROUTE=udp://127.0.0.1:52440 \
            ockam/telegraf-ockamd:0.1.0 > /dev/null
        ;;

    ockam-router)
        echo "TODO"
        ;;

    influxdb-query)
        # executes a 'SELECT * FROM temperature' query within the 'ockam_demo' database in 
        # `influxdb`.
        docker exec influxdb-ockamd influx -database "ockam_demo" -execute "select * from temperature"
        ;;

    telegraf-write)
        # sends a random 'temperature' measurement to `telegraf` agent, which is encrypted by 
        # `ockamd` and written to `influxdb`.
        TEMP=$(( ( RANDOM % 10 )  + 70 ))
        docker exec influxdb-ockamd \
            curl -s -X POST http://0.0.0.0:8080/telegraf \
            --data-binary "temperature,region=us-west temp=${TEMP}"
        ;;

    kill-all)
        # kills and removes all demo containers.
        docker container rm -f influxdb-ockamd
        docker container rm -f telegraf-ockamd
        ;;

esac
    