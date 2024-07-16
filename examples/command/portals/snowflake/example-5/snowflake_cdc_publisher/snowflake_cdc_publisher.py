import logging
import os
import time
import json
from datetime import datetime
import snowflake.connector
from confluent_kafka import Producer, KafkaException
import sys
import socket


# Environment variables
SNOWFLAKE_ACCOUNT = os.getenv("SNOWFLAKE_ACCOUNT")
SNOWFLAKE_HOST = os.getenv("SNOWFLAKE_HOST")
SNOWFLAKE_DATABASE = os.getenv("SNOWFLAKE_DATABASE")
SNOWFLAKE_SCHEMA = os.getenv("SNOWFLAKE_SCHEMA")
STREAM_NAME = os.getenv('STREAM_NAME')
BOOTSTRAP_SERVER= os.getenv('KAFKA_BOOTSTRAP_SERVERS')
SNOWFLAKE_WAREHOUSE = os.getenv('SNOWFLAKE_WAREHOUSE')
KAFKA_TOPIC_NAME = os.getenv("KAFKA_TOPIC_NAME", "cdc_events")
JOB_SUCCESS_SLEEP_TIME = int(os.getenv("JOB_SUCCESS_SLEEP_TIME", 60))
JOB_ERROR_SLEEP_TIME = int(os.getenv("JOB_ERROR_SLEEP_TIME", 120))
LOG_LEVEL = os.getenv("LOG_LEVEL", "INFO").upper()

logging.basicConfig(level=LOG_LEVEL, format='%(asctime)s - %(levelname)s - %(message)s')

def get_con():
    """
    Create and return a Snowflake connection.
    """
    connection_params = get_connection_params()
    return snowflake.connector.connect(**connection_params)

def get_connection_params():
    """
    Construct Snowflake connection params from environment variables.
    """
    if os.path.exists("/snowflake/session/token"):
        return {
            "account": SNOWFLAKE_ACCOUNT,
            "host": SNOWFLAKE_HOST,
            "authenticator": "oauth",
            "token": get_login_token(),
            "warehouse": SNOWFLAKE_WAREHOUSE,
            "database": SNOWFLAKE_DATABASE,
            "schema": SNOWFLAKE_SCHEMA,
            "ocsp_fail_open": True #https://docs.snowflake.com/en/developer-guide/python-connector/python-connector-connect#label-python-ocsp-choosing-fail-open-or-fail-close-mode
        }
    else:
        return {
            "account": SNOWFLAKE_ACCOUNT,
            "user": SNOWFLAKE_USER,
            "password": SNOWFLAKE_PASSWORD,
            "warehouse": SNOWFLAKE_WAREHOUSE,
            "database": SNOWFLAKE_DATABASE,
            "schema": SNOWFLAKE_SCHEMA
        }

def get_login_token():
    """
    Read the login token supplied automatically by Snowflake. These tokens
    are short lived and should always be read right before creating any new connection.
    """
    with open("/snowflake/session/token", "r") as f:
        return f.read()

def check_stream_exists(con, stream_name):
    with con.cursor() as cur:
        cur.execute(f"SHOW STREAMS LIKE '{stream_name.split('.')[-1]}'")
        if not cur.fetchone():
            raise Exception(f"Stream {stream_name} does not exist.")

def delivery_report(err, msg):
    if err is not None:
        logging.error(f'Message delivery failed: {err}')
    else:
        logging.info(f'Message delivered to {msg.topic()} [partition: {msg.partition()}] at offset {msg.offset()}')

def send_changes_to_kafka(producer, changes, kafka_topic_name):
    data = json.dumps({"changes": changes})
    logging.debug(f"Attempting to send data to Kafka: {data[:1000]}...")  # Log first 1000 chars of data
    try:
        producer.produce(kafka_topic_name, data.encode('utf-8'), callback=delivery_report)
        producer.poll(0)  # Trigger any callbacks
        logging.info(f"Successfully queued {len(changes)} changes for Kafka topic {kafka_topic_name}")
    except Exception as e:
        logging.error(f"Error sending data to Kafka: {e}")

    # Poll in a loop to trigger callbacks
    for i in range(30):  # Poll 30 times
        producer.poll(1)  # Poll for 1 second each time

    remaining = producer.flush(timeout=30)
    if remaining > 0:
        logging.warning(f"{remaining} messages were not delivered")
    else:
        logging.info(f"All {len(changes)} messages flushed successfully to Kafka topic {kafka_topic_name}")

def create_kafka_producer(bootstrap_servers):
    kafka_conf = {
        'bootstrap.servers': bootstrap_servers,
        'client.id': 'snowflake-change-publisher',
        'message.max.bytes': 1000000,
        'socket.timeout.ms': 10000,
        'message.timeout.ms': 30000,
        'metadata.request.timeout.ms': 15000,
        'max.in.flight.requests.per.connection': 1,
        'retries': 3,
        'retry.backoff.ms': 1000,
    }

    if LOG_LEVEL == 'DEBUG':
        kafka_conf['debug'] = 'broker,topic,msg'

    logging.info(f"Kafka configuration: {kafka_conf}")

    try:
        producer = Producer(kafka_conf)
        logging.info(f"Kafka producer created successfully")
        return producer
    except Exception as e:
        logging.error(f"Error creating Kafka producer: {str(e)}")
        raise

def wait_for_kafka(bootstrap_servers, retry_interval=10, max_retries=5):
    retries = 0
    while retries < max_retries:
        try:
            logging.info(f"Attempt {retries + 1}/{max_retries}: Connecting to Kafka broker at {bootstrap_servers}")
            producer = create_kafka_producer(bootstrap_servers)

            # Test the connection by listing topics
            cluster_metadata = producer.list_topics(timeout=10)
            logging.info(f"Successfully connected to Kafka. Cluster metadata: {cluster_metadata}")

            return producer
        except KafkaException as e:
            retries += 1
            logging.error(f"Kafka error: {str(e)}")
            if retries < max_retries:
                logging.info(f"Retrying in {retry_interval} seconds...")
                time.sleep(retry_interval)
            else:
                logging.error(f"Failed to connect to Kafka broker after {max_retries} attempts")
                raise
        except Exception as e:
            logging.error(f"Unexpected error: {str(e)}")
            raise

def read_stream(producer, stream_name, kafka_topic_name):
    logging.info(f"Starting to read stream {stream_name}")
    con = None
    try:
        con = get_con()
        check_stream_exists(con, stream_name)

        with con.cursor() as cur:
            cur.execute(f"CREATE TEMPORARY TABLE TMP AS SELECT * FROM {stream_name}")
            cur.execute("SELECT * FROM TMP")
            columns = [col[0] for col in cur.description]
            rows = cur.fetchall()

            changes = []
            for row in rows:
                change = dict(zip(columns, row))
                changes.append(change)
                logging.debug(f"Captured change: {change}")

            if changes:
                logging.info(f"Found {len(changes)} changes in stream {stream_name}")
                send_changes_to_kafka(producer, changes, kafka_topic_name)
            else:
                logging.info(f"No changes found in stream {stream_name}")

        con.commit()
    except Exception as e:
        logging.error(f"Error reading stream {stream_name}: {str(e)}")
        if con:
            con.rollback()
        raise
    finally:
        if con:
            con.close()

    logging.info(f"Sleeping for {JOB_SUCCESS_SLEEP_TIME} seconds")
    time.sleep(JOB_SUCCESS_SLEEP_TIME)

def print_environment_variables():
    relevant_vars = [
        'SNOWFLAKE_ACCOUNT', 'SNOWFLAKE_HOST', 'SNOWFLAKE_DATABASE', 'SNOWFLAKE_SCHEMA',
        'STREAM_NAME', 'KAFKA_TOPIC_NAME', 'KAFKA_BOOTSTRAP_SERVERS', 'SNOWFLAKE_WAREHOUSE',
        'JOB_SUCCESS_SLEEP_TIME', 'JOB_ERROR_SLEEP_TIME', 'LOG_LEVEL'
    ]

    logging.info("Environment Variables:")
    for var in relevant_vars:
        value = os.getenv(var, 'Not set')
        if var in globals():
            value = globals()[var]
        logging.info(f"{var}: {value}")


def main():
    try:
        print_environment_variables()

        producer = create_kafka_producer(BOOTSTRAP_SERVER)

        logging.info(f"Starting CDC process for stream {STREAM_NAME}")

        while True:
            try:
                logging.info(f"Processing stream {STREAM_NAME} to topic {KAFKA_TOPIC_NAME}")
                read_stream(producer, STREAM_NAME, KAFKA_TOPIC_NAME)
            except KafkaException as e:
                logging.error(f"Kafka error: {e}")
                # If Kafka becomes unavailable, we'll recreate the producer
                producer = create_kafka_producer(BOOTSTRAP_SERVER)
            except snowflake.connector.errors.ProgrammingError as e:
                logging.error(f"Snowflake Programming Error: {e}")
                time.sleep(JOB_ERROR_SLEEP_TIME)
            except snowflake.connector.errors.DatabaseError as e:
                logging.error(f"Snowflake Database Error: {e}")
                time.sleep(JOB_ERROR_SLEEP_TIME)
            except Exception as e:
                logging.error(f"Unexpected error in main loop: {e}")
                time.sleep(JOB_ERROR_SLEEP_TIME)

    except Exception as e:
        logging.error(f"Fatal error in main: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
