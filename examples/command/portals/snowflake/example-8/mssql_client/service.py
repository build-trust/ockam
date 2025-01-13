import logging
import os
import sys
import pymssql
import connection
from flask import Flask, request
from snowflake import connector
from snowflake.connector.errors import ProgrammingError, DatabaseError

# Environment variables
WAREHOUSE = os.getenv('SNOWFLAKE_WAREHOUSE', "reference('WAREHOUSE')")
LOG_LEVEL = os.getenv('LOG_LEVEL', 'INFO').upper()
logging.basicConfig(level=LOG_LEVEL, format='%(asctime)s - %(levelname)s - %(message)s')

app = Flask(__name__)

# MSSQL Configuration
MSSQL_USER = os.environ.get('MSSQL_USER')
MSSQL_PASSWORD = os.environ.get('MSSQL_PASSWORD')
MSSQL_DATABASE = os.environ.get('MSSQL_DATABASE')
MSSQL_SERVER = os.environ.get('ENDPOINT_HOST', 'localhost')
MSSQL_PORT = os.environ.get('ENDPOINT_PORT', '1433')

# Snowflake session
session = None

# MSSQL Connection
def get_mssql_connection():
    """Create MSSQL connection"""
    return pymssql.connect(
        server=MSSQL_SERVER,
        user=MSSQL_USER,
        password=MSSQL_PASSWORD,
        database=MSSQL_DATABASE,
        port=MSSQL_PORT
    )

def test_mssql_connection():
    """Test the MSSQL database connection"""
    try:
        with get_mssql_connection() as conn:
            with conn.cursor() as cursor:
                cursor.execute("SELECT @@VERSION AS SQLServerVersion;")
                version = cursor.fetchone()[0]
                logging.info(f"Successfully connected to SQL Server")
                logging.info(f"Server Version: {version}")
    except Exception as e:
        logging.error(f"Connection test failed: {str(e)}")
        raise

# Snowflake Query
def execute_snowflake_query(query, values=[]) -> bool:
    """Execute a Snowflake SQL query"""
    try:
        logging.info(f"Executing query: {query}, with values {values}")
        session.sql(query, values).collect()
        logging.info(f"Query execution successful")
        return True
    except (ProgrammingError, DatabaseError) as e:
        logging.error(f"Snowflake Error: {type(e).__name__} - {str(e)}")
        return False
    except Exception as e:
        logging.error(f"Unexpected error executing query: {type(e).__name__} - {str(e)}")
        return False

def use_snowflake_referenced_warehouse():
    """Use the warehouse referenced as 'WAREHOUSE' for the current session"""
    try:
        logging.info(f"Use the referenced warehouse")
        result = session.sql(f"USE WAREHOUSE {WAREHOUSE}").collect()
        logging.info(f"Result of USE WAREHOUSE: {result}.")
    except Exception as e:
        logging.error(f"Cannot use the referenced warehouse: {type(e).__name__} - {str(e)}")
        raise

# Snowflake Copy
@app.route("/copy_to_snowflake", methods=["POST"])
def copy_to_snowflake():
    message = request.json
    logging.info(f"Received message: {message}")

    source_table = message['data'][0][1]
    target_table = message['data'][0][2]

    logging.info(f"Copying from {source_table} to {target_table}")

    try:
        # Get data from MSSQL
        with get_mssql_connection() as conn:
            with conn.cursor() as cursor:
                cursor.execute(f"SELECT * FROM {source_table}")
                columns = [column[0] for column in cursor.description]
                rows = [list(row) for row in cursor.fetchall()]

                # Format data for Snowflake
                columns_str = ", ".join(columns)
                values_list = []
                for row in rows:
                    values = [f"'{str(val)}'" if isinstance(val, str) else str(val) for val in row]
                    values_list.append(f"({', '.join(values)})")

                # Insert data into Snowflake
                insert_sql = f"INSERT INTO {target_table} ({columns_str}) VALUES {', '.join(values_list)}"
                result = execute_snowflake_query(insert_sql)
                if result is False:
                    error_msg = "Failed to insert data into target table. Check service logs for more details."
                    logging.error(error_msg)
                    return {
                        'data': [[0, error_msg]]
                    }

                rows_inserted = len(rows)
                return {
                    'data': [[0, f"Successfully copied {rows_inserted} rows from {source_table} to {target_table}"]]
                }
    except Exception as e:
        error_msg = str(e)
        logging.error(f"Error: {error_msg}")
        return {
            'data': [[0, f"Error: {error_msg}"]]
        }

def main():
    global session
    # Connect to Snowflake
    session = connection.session()
    try:
        logging.info(f"Start the server")
        use_snowflake_referenced_warehouse()
        test_mssql_connection()
        app.run(host='0.0.0.0', port=8080)
    except Exception as e:
        logging.error(f"Fatal error in main: {e}")
        sys.exit(1)

if __name__ == '__main__':
    main()
