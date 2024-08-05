import logging
import os
import time
import json
import sys
import snowflake.connector
from snowflake.connector.errors import ProgrammingError, DatabaseError
import connection
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

session = connection.session()

session.use_database(SNOWFLAKE_DATABASE)
session.use_schema(SNOWFLAKE_SCHEMA)

def main():
    try:
        logging.info(f"Starting Kafka to Snowflake ingestion process")
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

def insert_into_snowflake(session, table, key, value):
    try:
        # Prepare the query with placeholders for parameters
        query = f"INSERT INTO {table} (KEY, VALUE) VALUES (%s, %s)"
        
        # Properly format the SQL query using Python's string formatting
        query = query % (f"'{key}'", f"'{value}'")
        
        # Execute the query using session.sql() and collect()
        session.sql(query).collect()
        
        logging.info(f"Successfully inserted message into {table} - Key: {key}, Value: {value[:20]}...")
    except (ProgrammingError, DatabaseError) as e:
        logging.error(f"Snowflake Error: {type(e).__name__} - {str(e)}")
        logging.error(f"Failed SQL: {query}")
        logging.error(f"Parameters - Key: {key}, Value: {value}")
    except Exception as e:
        logging.error(f"Unexpected error inserting into Snowflake: {type(e).__name__} - {str(e)}")
        logging.error(f"Failed SQL: {query}")
        logging.error(f"Parameters - Key: {key}, Value: {value}")


def consume_events(session, consumer, kafka_topic):
    logging.info(f"Starting to consume events from topic {kafka_topic} for table {TARGET_TABLE}")
    check_if_target_table_exists(session, TARGET_TABLE)

    messages_processed = 0
    last_message_time = time.time()

    try:
        while True:
            msg = consumer.poll(1.0)

            if msg is None:
                current_time = time.time()
                if current_time - last_message_time > JOB_SUCCESS_SLEEP_TIME:
                    logging.info(f"No new messages for {JOB_SUCCESS_SLEEP_TIME} seconds. Total messages processed: {messages_processed}")
                    last_message_time = current_time
                continue

            if msg.error():
                if msg.error().code() == KafkaError._PARTITION_EOF:
                    logging.warning(f'Reached end of partition for topic {msg.topic()}, partition {msg.partition()}')
                else:
                    logging.error(f'Error while consuming message: {msg.error()}')
            else:
                try:
                    key = msg.key().decode('utf-8') if msg.key() else "None"
                    value = msg.value().decode('utf-8') if msg.value() else "None"
                    logging.info(f"Received message:")
                    logging.info(f"  Topic: {msg.topic()}")
                    logging.info(f"  Partition: {msg.partition()}")
                    logging.info(f"  Offset: {msg.offset()}")
                    logging.info(f"  Key: {key}")
                    logging.info(f"  Value: {value[:100]}...")  # Log first 100 chars of the value
                    
                    # Insert into Snowflake table
                    insert_into_snowflake(session, TARGET_TABLE, key, value)
                    
                    messages_processed += 1
                    logging.info(f"Successfully processed message. Total processed: {messages_processed}")
                    
                    last_message_time = time.time()
                    
                    # Commit the offset
                    consumer.commit(msg)
                    logging.info(f"Committed offset {msg.offset()} for partition {msg.partition()}")
                except Exception as e:
                    logging.error(f"Error processing message: {str(e)}")

    except Exception as e:
        logging.error(f"Error in consume_events: {str(e)}")
    finally:
        logging.info(f"Closing consumer. Total messages processed: {messages_processed}")
        consumer.close()


def check_if_target_table_exists(session, target_table_name):
    try:
        table_parts = target_table_name.split('.')
        if len(table_parts) != 3:
            raise ValueError(f"Invalid table name format: {target_table_name}. Expected format: DATABASE.SCHEMA.TABLE")
        
        database, schema, table = table_parts
        stream = session.sql(f"SHOW TABLES LIKE '{table}' IN {database}.{schema}").collect()

        if not stream:
            raise Exception(f"The table {target_table_name} does not exist or is not accessible. Please check the 'target_table' reference.")
    except Exception as e:
        logging.error(f"Error checking target table: {e}")
        raise


def wait_for_kafka(bootstrap_servers, retry_interval=JOB_ERROR_SLEEP_TIME, max_retries=10):
    retries = 0
    while retries < max_retries:
        try:
            logging.info(f"Attempt {retries + 1}/{max_retries}: Connecting to Kafka broker at {bootstrap_servers}")
            consumer = create_kafka_consumer(bootstrap_servers)

            # Test the connection by listing topics
            cluster_metadata = consumer.list_topics(timeout=100)
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
        'TARGET_TABLE',
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