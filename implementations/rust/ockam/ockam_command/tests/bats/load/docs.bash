#!/bin/bash

start_python_server() {
  pushd $OCKAM_HOME

  cat >main.py <<-EOM
import os
import psycopg2
from flask import Flask
CREATE_TABLE = (
  "CREATE TABLE IF NOT EXISTS events (id SERIAL PRIMARY KEY, name TEXT);"
)
INSERT_RETURN_ID = "INSERT INTO events (name) VALUES (%s) RETURNING id;"
app = Flask(__name__)
pg_port = os.environ['APP_PG_PORT']
url = "postgres://postgres:password@localhost:%s/"%pg_port
connection = psycopg2.connect(url)

@app.route("/")
def hello_world():
  with connection:
    with connection.cursor() as cursor:
        cursor.execute(CREATE_TABLE)
        cursor.execute(INSERT_RETURN_ID, ("",))
        id = cursor.fetchone()[0]
  return "I've been visited {} times".format(id), 201
EOM

  flask --app main run -p "$FLASK_PORT" &>>$OCKAM_HOME/file.log &
  pid="$!"
  echo "$pid" >"flask.pid"
  sleep 5
  popd
}

kill_flask_server() {
  pid=$(cat "${OCKAM_HOME}/flask.pid")
  kill -9 "$pid" || true
  wait "$pid" 2>>/dev/null || true
}

kill_kafka_contents() {
  kafka-topics.sh --bootstrap-server localhost:4000 --command-config "$KAFKA_CONFIG" --delete --topic $DEMO_TOPIC || true

  pid=$(cat "$ADMIN_HOME/kafka.pid") || return
  kill -9 "$pid"
  wait "$pid" 2>>/dev/null || true
}

start_telegraf_instance() {
  telegraf_conf="$(mktemp)/telegraf.conf"

  cat >$telegraf_conf <<EOF
[[outputs.influxdb_v2]]
  urls = ["http://127.0.0.1:${INFLUX_PORT}"]
  token = "${INFLUX_TOKEN}"
  organization = "${INFLUX_ORG}"
  bucket = "${INFLUX_BUCKET}"

[[inputs.cpu]]
EOF

  telegraf --config $telegraf_conf &
  pid="$!"
  echo "$pid" >"${ADMIN_HOME}/telegraf.pid"
  sleep 5
}

kill_telegraf_instance() {
  pid=$(cat "${ADMIN_HOME}/telegraf.pid") || return
  kill -9 "$pid"
  wait "$pid" 2>>/dev/null || true
}
