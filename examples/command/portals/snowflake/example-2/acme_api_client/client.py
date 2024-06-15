import requests
import time
import random
import logging
import os
import snowflake.connector
from snowflake.snowpark import Session
from datetime import datetime

# Environment variables below will be automatically populated by Snowflake.
SNOWFLAKE_ACCOUNT = os.getenv("SNOWFLAKE_ACCOUNT")
SNOWFLAKE_HOST = os.getenv("SNOWFLAKE_HOST")
SNOWFLAKE_DATABASE = os.getenv("SNOWFLAKE_DATABASE")
SNOWFLAKE_SCHEMA = os.getenv("SNOWFLAKE_SCHEMA")

# User defined
SNOWFLAKE_WAREHOUSE = os.getenv("SNOWFLAKE_WAREHOUSE")

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')

# Get the protocol, host, and port from environment variables
protocol = os.getenv('ENDPOINT_PROTOCOL', 'http')
host = os.getenv('ENDPOINT_HOST', 'localhost')
port = os.getenv('ENDPOINT_PORT', '15000')

# Construct the base URL
base_url = f"{protocol}://{host}:{port}"
# List of available endpoints
endpoints = ["/", "/ping", "/greet", "/farewell"]


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
            "host": SNOWFLAKE_HOST,
            "user": SNOWFLAKE_USER,
            "password": SNOWFLAKE_PASSWORD,
            "role": SNOWFLAKE_ROLE,
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

def send_request():
    # Select a random endpoint
    endpoint = random.choice(endpoints)
    url = f"{base_url}{endpoint}"

    try:
        # Send a GET request to the selected endpoint
        response = requests.get(url)
        # Log the response text
        logging.info(f"Hit {url}: Response -> {response.text}")


        input_text = endpoint
        response_text = response.text
        timestamp = datetime.now().strftime('%H:%M:%S') # Format time to match the Snowflake TIME type

        query = "INSERT INTO API_RESULTS (API_INPUT, API_RESPONSE, TIME) VALUES (?, ?, ?)"
        values = (input_text, response_text, timestamp)
        result_table = "API_RESULTS"

        # SQL statement to insert data directly
        with Session.builder.configs(get_connection_params()).create() as session:
            # Print out current session context information.
            database = session.get_current_database()
            schema = session.get_current_schema()
            warehouse = session.get_current_warehouse()
            role = session.get_current_role()
            logging.info("Host:")
            logging.info(SNOWFLAKE_HOST)
            logging.info(
                f"Connection succeeded. Current session context: database={database}, schema={schema}, warehouse={warehouse}, role={role}"
            )
            # Execute query
            logging.info(
                f"Executing query [{query}]"
            )
            session.sql(query, values).collect()

    except requests.exceptions.ConnectionError as e:
        logging.error(f"Failed to connect to {url}: {e}")

if __name__ == "__main__":
    while True:
        send_request()
        # Wait for 30 seconds before making the next request
        time.sleep(10)
