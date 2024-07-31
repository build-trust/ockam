import logging
import os
import time
import sys
import psycopg2
import connection

# Environment variables
SNOWFLAKE_DATABASE = os.getenv('SNOWFLAKE_DATABASE')
SNOWFLAKE_SCHEMA = os.getenv('SNOWFLAKE_SCHEMA')
POSTGRES_HOST = os.getenv('POSTGRES_HOST')
POSTGRES_PORT = os.getenv('POSTGRES_PORT')
POSTGRES_USER = os.getenv('POSTGRES_USER')
POSTGRES_PASSWORD = os.getenv('POSTGRES_PASSWORD', '')
POSTGRES_DATABASE = os.getenv('POSTGRES_DATABASE', 'postgres')
BOOTSTRAP_SERVERS= os.getenv('KAFKA_BOOTSTRAP_SERVERS')
JOB_SUCCESS_SLEEP_TIME = int(os.getenv('JOB_SUCCESS_SLEEP_TIME', 2))
JOB_ERROR_SLEEP_TIME = int(os.getenv('JOB_ERROR_SLEEP_TIME', 2))
LOG_LEVEL = os.getenv('LOG_LEVEL', 'INFO').upper()

logging.basicConfig(level=LOG_LEVEL, format='%(asctime)s - %(levelname)s - %(message)s')

# Create a Snowflake session
snowflake_session = connection.session()


def main():
    try:
        logging.info(f"Connecting to a private Postgres database via Ockam")
        print_environment_variables()

        while True:
            try:
                postgres_connection = create_postgres_connection()
                cursor = postgres_connection.cursor()
                print("Connected to the database")
            except Exception as e:
                logging.error(f"Unexpected error in main loop: {e}")
                time.sleep(JOB_ERROR_SLEEP_TIME)

    except Exception as e:
        logging.error(f"Fatal error in main: {e}")
        sys.exit(1)


# Connect to the Postgres endpoint provided the deployed Ockam node
def create_postgres_connection():
    try:
        logging.info(f"Connecting to Postgres...")
        postgres_connection = psycopg2.connect(
            user=POSTGRES_USER,
            host=POSTGRES_HOST,
            port=POSTGRES_PORT,
            database=POSTGRES_DATABASE,
        )
        logging.info(f"Postgres connection created successfully")
        return postgres_connection
    except Exception as e:
        logging.error(f"Error creating a connection to Postgres: {str(e)}")
        raise


# Print the list of environment variables to check the application setup
def print_environment_variables():
    relevant_vars = [
        'SNOWFLAKE_ACCOUNT',
        'SNOWFLAKE_WAREHOUSE',
        'SNOWFLAKE_HOST',
        'SNOWFLAKE_DATABASE',
        'SNOWFLAKE_SCHEMA',
        'SNOWFLAKE_ROLE',
        'SNOWFLAKE_USER',
        'JOB_SUCCESS_SLEEP_TIME',
        'JOB_ERROR_SLEEP_TIME',
        'LOG_LEVEL',
        'HOSTNAME',
        'POSTGRES_HOST',
        'POSTGRES_PORT',
        'POSTGRES_USER',
        'POSTGRES_PASSWORD',
        'POSTGRES_DATABASE',
    ]

    logging.info("Application environment variables:")
    for var in relevant_vars:
        value = os.getenv(var, 'Not set')
        if var in globals():
            value = globals()[var]
        logging.info(f"{var}: {value}")


if __name__ == "__main__":
    main()
