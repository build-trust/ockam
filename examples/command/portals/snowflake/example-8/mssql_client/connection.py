import os
import logging
import snowflake.connector
from snowflake.snowpark import Session


def session() -> Session:
    """
    Create a session for the connection
    :return: Session
    """
    logging.info(f"Create a session")
    return Session.builder.configs({"connection": connection()}).create()


def connection() -> snowflake.connector.SnowflakeConnection:
    """
    Create a connection, either from inside the native application when deployed
    or with user/password credentials when testing locally

    :return: A SnowflakeConnection
    """
    logging.info(f"Create a connection")
    if os.path.isfile("/snowflake/session/token"):
        logging.info(f"Use an OAUTH token")
        creds = {
            'account': os.getenv('SNOWFLAKE_ACCOUNT'),
            'host': os.getenv('SNOWFLAKE_HOST'),
            'port': os.getenv('SNOWFLAKE_PORT'),
            'protocol': 'https',
            'warehouse': os.getenv('SNOWFLAKE_WAREHOUSE'),
            'database': os.getenv('SNOWFLAKE_DATABASE'),
            'schema': os.getenv('SNOWFLAKE_SCHEMA'),
            'role': os.getenv('SNOWFLAKE_ROLE'),
            'authenticator': "oauth",
            'token': open('/snowflake/session/token', 'r').read(),
            'client_session_keep_alive': True,
            'ocsp_fail_open': False,
            'validate_default_parameters': True,
        }
        #logging.info(f"the creds are {creds}")
    else:
        creds = {
            'account': os.getenv('SNOWFLAKE_ACCOUNT'),
            'user': os.getenv('SNOWFLAKE_USER'),
            'password': os.getenv('SNOWFLAKE_PASSWORD'),
            'warehouse': os.getenv('SNOWFLAKE_WAREHOUSE'),
            'database': os.getenv('SNOWFLAKE_DATABASE'),
            'schema': os.getenv('SNOWFLAKE_SCHEMA'),
            'client_session_keep_alive': True
        }

    return snowflake.connector.connect(**creds)
