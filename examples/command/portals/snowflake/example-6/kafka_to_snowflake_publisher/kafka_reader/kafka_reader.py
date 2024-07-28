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
TARGET_TABLE = os.getenv('TARGET_TABLE')

logging.basicConfig(level=LOG_LEVEL, format='%(asctime)s - %(levelname)s - %(message)s')

import connection
session = connection.session()

def main():
    try:
        logging.info(f"Starting to consume Kafka events for topic {KAFKA_TOPIC}")
        print_environment_variables()
        consumer = create_kafka_consumer(BOOTSTRAP_SERVERS)
        
        check_topic_details(consumer, KAFKA_TOPIC)
        
        consume_events(session, consumer, KAFKA_TOPIC)

    except Exception as e:
        logging.error(f"Fatal error in main: {e}")
        sys.exit(1)

def create_kafka_consumer(bootstrap_servers):
    kafka_configuration = {
        'bootstrap.servers': bootstrap_servers,
        'client.id': 'kafka_ingest',
        'group.id': 'kafka_ingest_group',
        'auto.offset.reset': 'earliest',
        'enable.auto.commit': False,
        'api.version.request': True,
        'api.version.fallback.ms': 0,
        'session.timeout.ms': 10000,
        'max.poll.interval.ms': 300000,
        'socket.timeout.ms': 10000,
    }

    kafka_configuration['debug'] = 'consumer,cgrp,topic,fetch'

    logging.info(f"Kafka configuration: {kafka_configuration}")

    try:
        consumer = Consumer(kafka_configuration)
        logging.info(f"Kafka consumer created successfully")
        logging.info(f"Subscribing to Kafka topic: {KAFKA_TOPIC}")
        consumer.subscribe([KAFKA_TOPIC], on_assign=print_assignment)
        return consumer
    except Exception as e:
        logging.error(f"Error creating Kafka consumer: {str(e)}")
        raise
def check_topic_details(consumer, topic):
    try:
        cluster_metadata = consumer.list_topics(topic, timeout=5)
        if topic not in cluster_metadata.topics:
            logging.error(f"Topic '{topic}' does not exist.")
            return

        topic_metadata = cluster_metadata.topics[topic]
        partitions = topic_metadata.partitions
        logging.info(f"Topic '{topic}' has {len(partitions)} partitions")

        for partition_id, partition_metadata in partitions.items():
            try:
                low, high = consumer.get_watermark_offsets(TopicPartition(topic, partition_id), timeout=5)
                logging.info(f"Partition {partition_id}: Low offset = {low}, High offset = {high}")
            except Exception as e:
                logging.error(f"Error getting offsets for partition {partition_id}: {e}")

    except Exception as e:
        logging.error(f"Error checking topic details: {e}")

def print_assignment(consumer, partitions):
    logging.info(f"Consumer assignment: {partitions}")

def consume_events(session, consumer, kafka_topic):
    logging.info(f"Ingesting events into the table {TARGET_TABLE}")

    try:
        while True:
            msg = consumer.poll(5.0)  # Increased timeout to 5 seconds

            if msg is None:
                logging.info("No message received.")
                continue

            if msg.error():
                if msg.error().code() == KafkaError._PARTITION_EOF:
                    logging.warning(f'Reached end of partition for topic {msg.topic()}, partition {msg.partition()}')
                else:
                    logging.error(f'Error while consuming message: {msg.error()}')
            else:
                logging.info(f"Received message: topic={msg.topic()}, partition={msg.partition()}, offset={msg.offset()}, key={msg.key()}, value={msg.value().decode('utf-8')}")

            # Print current assignment and position after each poll
            assignment = consumer.assignment()
            for tp in assignment:
                position = consumer.position(tp)
                logging.info(f"Current assignment - Topic: {tp.topic}, Partition: {tp.partition}, Current Position: {position}")

    except KeyboardInterrupt:
        logging.info("Interrupted by user. Closing consumer.")
    finally:
        consumer.close()
        logging.info("Consumer closed.")


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