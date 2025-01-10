import logging
import os
import sys
import pymssql
from flask import request
from flask import Flask

# Environment variables
LOG_LEVEL = os.getenv('LOG_LEVEL', 'DEBUG').upper()
logging.basicConfig(level=LOG_LEVEL, format='%(asctime)s - %(levelname)s - %(message)s')

app = Flask(__name__)

MSSQL_USER = os.environ.get('MSSQL_USER')
MSSQL_PASSWORD = os.environ.get('MSSQL_PASSWORD')
MSSQL_DATABASE = os.environ.get('MSSQL_DATABASE')
MSSQL_SERVER = os.environ.get('ENDPOINT_HOST', 'localhost')

# Create connection function
def get_connection():
    return pymssql.connect(
        server=MSSQL_SERVER,
        user=MSSQL_USER,
        password=MSSQL_PASSWORD,
        database=MSSQL_DATABASE
    )

@app.route("/query", methods=["POST"])
def query():
    message = request.json
    logging.info(f"Received message: {message}")
    user_query = message['data'][0][1]
    logging.info(f"Received query: {user_query}")

    with get_connection() as conn:
        with conn.cursor() as cursor:
            cursor.execute(user_query)
            columns = [column[0] for column in cursor.description]
            rows = [list(row) for row in cursor.fetchall()]
            rows = [columns] + rows
            data = {
                'data': [[0, rows]],
            }
    logging.info(f"Returning data: {data}")
    return data

@app.route("/execute", methods=["POST"])
def execute():
    message = request.json
    logging.info(f"Received message: {message}")
    user_query = message['data'][0][1]
    logging.info(f"Received query: {user_query}")

    with get_connection() as conn:
        with conn.cursor() as cursor:
            cursor.execute(user_query)
            conn.commit()
    return {
        'data': [[0, "SUCCESS"]],
    }

@app.route("/insert", methods=["POST"])
def insert():
    message = request.json
    logging.info(f"Received message: {message}")
    user_query = message['data'][0][1]
    logging.info(f"Received query: {user_query}")
    values = message['data'][0][2]
    logging.info(f"Received values: {values}")

    if len(values) == 0:
        return {
            'data': [[0, "SUCCESS"]],
        }

    with get_connection() as conn:
        with conn.cursor() as cursor:
            for value in values:
                cursor.execute(user_query, value)
            conn.commit()
    return {
        'data': [[0, "SUCCESS"]],
    }

@app.route("/ready", methods=["GET"])
def ready():
    return {}

def print_environment_variables():
    """
    Print the relevant environment variables for diagnostic
    """
    relevant_vars = [
        'SNOWFLAKE_ACCOUNT',
        'SNOWFLAKE_WAREHOUSE',
        'SNOWFLAKE_HOST',
        'SNOWFLAKE_DATABASE',
        'SNOWFLAKE_SCHEMA',
        'SNOWFLAKE_ROLE',
        'SNOWFLAKE_USER',
        'LOG_LEVEL',
        'MSSQL_DATABASE',
    ]

    logging.info("Application environment variables:")
    for var in relevant_vars:
        value = os.getenv(var, 'Not set')
        if var in globals():
            value = globals()[var]
        logging.info(f"{var}: {value}")

def test_connection():
    """Test the database connection with a simple system query"""
    try:
        with get_connection() as conn:
            with conn.cursor() as cursor:
                cursor.execute("SELECT @@VERSION AS SQLServerVersion;")
                version = cursor.fetchone()[0]
                logging.info(f"Successfully connected to SQL Server")
                logging.info(f"Server Version: {version}")
    except Exception as e:
        logging.error(f"Connection test failed: {str(e)}")

if __name__ == "__main__":
    print_environment_variables()
    test_connection()
    app.run(host='0.0.0.0', port=8080)
