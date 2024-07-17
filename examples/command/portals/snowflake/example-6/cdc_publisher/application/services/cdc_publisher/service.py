import logging
import os
import time
import json
import sys
import snowflake.connector
from confluent_kafka import Producer, KafkaException


# Environment variables
SNOWFLAKE_DATABASE = os.getenv('SNOWFLAKE_DATABASE')
SNOWFLAKE_SCHEMA = os.getenv('SNOWFLAKE_SCHEMA')
BOOTSTRAP_SERVER= os.getenv('KAFKA_BOOTSTRAP_SERVERS')
KAFKA_TOPIC_NAME = os.getenv('KAFKA_TOPIC_NAME')
JOB_SUCCESS_SLEEP_TIME = int(os.getenv('JOB_SUCCESS_SLEEP_TIME', 60))
JOB_ERROR_SLEEP_TIME = int(os.getenv('JOB_ERROR_SLEEP_TIME', 120))
LOG_LEVEL = os.getenv('LOG_LEVEL', 'INFO').upper()
STREAM_NAME = 'cdc_stream'

logging.basicConfig(level=LOG_LEVEL, format='%(asctime)s - %(levelname)s - %(message)s')

import connection
session = connection.session()

def main():
    try:
        logging.info(f"Starting the CDC process for stream {STREAM_NAME}")
        print_environment_variables()
        producer = create_kafka_producer(BOOTSTRAP_SERVER)
        while True:
            try:
                read_stream(session, producer, STREAM_NAME, KAFKA_TOPIC_NAME)
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

def create_kafka_producer(bootstrap_servers):
    kafka_configuration = {
        'bootstrap.servers': bootstrap_servers,
        'client.id': 'cdc-publisher',
        'message.max.bytes': 1000000,
        'socket.timeout.ms': 10000,
        'message.timeout.ms': 30000,
        'metadata.request.timeout.ms': 15000,
        'max.in.flight.requests.per.connection': 1,
        'retries': 3,
        'retry.backoff.ms': 1000,
    }

    if LOG_LEVEL == 'DEBUG':
        kafka_configuration['debug'] = 'broker,topic,msg'

    logging.info(f"Kafka configuration: {kafka_configuration}")

    try:
        producer = Producer(kafka_configuration)
        logging.info(f"Kafka producer created successfully")
        return producer
    except Exception as e:
        logging.error(f"Error creating Kafka producer: {str(e)}")
        raise

def read_stream(session, producer, stream_name, kafka_topic_name):
    logging.info(f"Starting to read the stream {stream_name}")
    check_stream_exists(session, stream_name)

    try:
        session.sql("BEGIN")

        session.sql(f"CREATE TEMPORARY TABLE TMP AS SELECT * FROM {stream_name}")
        stream_metadata = session.sql(f"""
                 SELECT COLUMN_NAME
                 FROM INFORMATION_SCHEMA.COLUMNS
                 WHERE TABLE_NAME = 'TMP'
                 AND TABLE_SCHEMA = '{SNOWFLAKE_SCHEMA.upper()}'
                 AND TABLE_CATALOG = '{SNOWFLAKE_DATABASE.upper()}'
             """).collect()
        column_names = [row['COLUMN_NAME'] for row in stream_metadata]
        rows = session.sql("SELECT * FROM TMP").collect()

        changes = []
        for row in rows:
            change = dict(zip(column_names, row))
            changes.append(change)
            logging.debug(f"Captured change: {change}")

        if changes:
            logging.info(f"Found {len(changes)} changes in stream {stream_name}")
            send_changes_to_kafka(producer, changes, kafka_topic_name)
        else:
            logging.info(f"No changes found in stream {stream_name}")
        session.sql("COMMIT")
    except Exception as e:
        logging.error(f"Error reading stream {stream_name}: {str(e)}")
        session.sql("ROLLBACK")
        raise

    logging.info(f"Sleeping for {JOB_SUCCESS_SLEEP_TIME} seconds")
    time.sleep(JOB_SUCCESS_SLEEP_TIME)

def check_stream_exists(session, stream_name):
    stream = session.sql(f"SHOW STREAMS LIKE '{stream_name.split('.')[-1]}'").collect()

    if not stream:
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

def print_environment_variables():
    relevant_vars = [
        'SNOWFLAKE_ACCOUNT',
        'SNOWFLAKE_WAREHOUSE',
        'SNOWFLAKE_HOST',
        'SNOWFLAKE_DATABASE',
        'SNOWFLAKE_SCHEMA',
        'SNOWFLAKE_ROLE',
        'SNOWFLAKE_USER',
        'STREAM_NAME',
        'KAFKA_TOPIC_NAME',
        'KAFKA_BOOTSTRAP_SERVERS',
        'JOB_SUCCESS_SLEEP_TIME',
        'JOB_ERROR_SLEEP_TIME',
        'LOG_LEVEL'
    ]

    logging.info("Environment Variables:")
    for var in relevant_vars:
        value = os.getenv(var, 'Not set')
        if var in globals():
            value = globals()[var]
        logging.info(f"{var}: {value}")


if __name__ == "__main__":
    main()
