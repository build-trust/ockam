import logging
import os
import time
import json
import sys
import snowflake.connector
from confluent_kafka import Consumer, KafkaException, KafkaError


# Environment variables
SNOWFLAKE_DATABASE = os.getenv('SNOWFLAKE_DATABASE')
SNOWFLAKE_SCHEMA = os.getenv('SNOWFLAKE_SCHEMA')
BOOTSTRAP_SERVERS= os.getenv('KAFKA_BOOTSTRAP_SERVERS')
KAFKA_TOPIC = os.getenv('KAFKA_TOPIC')
JOB_SUCCESS_SLEEP_TIME = int(os.getenv('JOB_SUCCESS_SLEEP_TIME', 60))
JOB_ERROR_SLEEP_TIME = int(os.getenv('JOB_ERROR_SLEEP_TIME', 120))
LOG_LEVEL = os.getenv('LOG_LEVEL', 'INFO').upper()
TARGET_TABLE = 'target_table'

logging.basicConfig(level=LOG_LEVEL, format='%(asctime)s - %(levelname)s - %(message)s')

import connection
session = connection.session()

def main():
    try:
        logging.info(f"Starting to consume Kafka events for topic {KAFKA_TOPIC}")
        print_environment_variables()
        wait_for_kafka(BOOTSTRAP_SERVERS)
        consumer = create_kafka_consumer(BOOTSTRAP_SERVERS)

        while True:
            try:
                consume_events(session, consumer, KAFKA_TOPIC)
            except KafkaException as e:
                logging.error(f"Kafka error: {e}")
                # If Kafka becomes unavailable, we'll recreate the consumer
                consumer = create_kafka_consumer(BOOTSTRAP_SERVERS)
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

def create_kafka_consumer(bootstrap_servers):
    kafka_configuration = {
        'bootstrap.servers': bootstrap_servers,
        'client.id': 'kafka_ingest',
        'group.id': 'kafka_ingest',
        'auto.offset.reset': 'latest',
        'api.version.request': False,
        'retries': 15,
        'retry.backoff.ms': 1000,
    }

    if LOG_LEVEL == 'DEBUG':
        kafka_configuration['debug'] = 'broker,topic,msg'

    logging.info(f"Kafka configuration: {kafka_configuration}")

    try:
        consumer = Consumer(kafka_configuration)
        logging.info(f"Kafka consumer created successfully")
        consumer.subscribe([KAFKA_TOPIC])
        return consumer
    except Exception as e:
        logging.error(f"Error creating Kafka consumer: {str(e)}")
        raise

def consume_events(session, consumer, kafka_topic):
    logging.info(f"Ingesting events into the table {TARGET_TABLE}")
    check_if_target_table_exists(session, TARGET_TABLE)

    try:
        while True:
            # Wait for message or event/error for 1 second
            msg = consumer.poll(1.0)

            if msg is None:
                # If no message available within timeout, wait a bit then continue polling.
                logging.info(f"Sleeping for {JOB_SUCCESS_SLEEP_TIME} seconds")
                time.sleep(JOB_SUCCESS_SLEEP_TIME)
                continue

            # End of partition event
            if msg.error():
                if msg.error().code() == KafkaError._PARTITION_EOF:
                    logging.warning('Reached end of partition')
                else:
                    logging.error('Error while consuming message: {}'.format(msg.error()))
            else:
                logging.info('Received message: {}'.format(msg.value().decode('utf-8')))


    finally:
        consumer.close()


def check_if_target_table_exists(session, target_table_name):
    stream = session.sql(f"SHOW TABLE LIKE '{target_table_name.split('.')[-1]}'").collect()

    if not stream:
        raise Exception(f"The table {target_table_name} does not exist or is not accessible. Please check the 'target_table' reference.")

def wait_for_kafka(bootstrap_servers, retry_interval=10, max_retries=5):
    retries = 0
    while retries < max_retries:
        try:
            logging.info(f"Attempt {retries + 1}/{max_retries}: Connecting to Kafka broker at {bootstrap_servers}")
            consumer = create_kafka_consumer(bootstrap_servers)

            # Test the connection by listing topics
            cluster_metadata = consumer.list_topics(timeout=10)
            logging.info(f"Successfully connected to Kafka. Cluster metadata: {cluster_metadata}")
            return consumer

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
        'KAFKA_TOPIC',
        'KAFKA_BOOTSTRAP_SERVERS',
        'JOB_SUCCESS_SLEEP_TIME',
        'JOB_ERROR_SLEEP_TIME',
        'LOG_LEVEL',
        'HOSTNAME',
    ]

    logging.info("Application environment variables:")
    for var in relevant_vars:
        value = os.getenv(var, 'Not set')
        if var in globals():
            value = globals()[var]
        logging.info(f"{var}: {value}")


if __name__ == "__main__":
    main()
