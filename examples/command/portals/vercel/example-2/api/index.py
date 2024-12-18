"""
Simple API handler for accessing PostgreSQL via Ockam secure channel
"""

import json
import os
import subprocess
import threading
import time
from decimal import Decimal
from http.server import BaseHTTPRequestHandler

import psycopg2
from psycopg2.extensions import ISOLATION_LEVEL_AUTOCOMMIT

# Constants
# TODO: Use ockam_inmemory testing. Cleanup after testing
OCKAM_BINARY_PATH = '/var/task/data/linux-x86_64/ockam' if os.environ.get('VERCEL_ENV') == 'production' else 'ockam'
MAX_RETRIES = 60
RETRY_DELAY = 1  # seconds
DB_PORT = 15432
DB_TIMEOUT = 3  # seconds

# Track if Ockam node creation has been attempted
NODE_INITIALIZED = False


def is_production() -> bool:
    """Check if the environment is production."""
    return os.environ.get('VERCEL_ENV') == 'production'


def get_ockam_version() -> str:
    """Get the installed Ockam version."""
    try:
        ockam_path = OCKAM_BINARY_PATH
        if is_production() and not os.path.exists(ockam_path):
            return f"Error: Ockam binary not found at {os.path.abspath(ockam_path)}"

        result = subprocess.run([ockam_path, '--version'],
                                capture_output=True,
                                text=True)

        return result.stdout.strip() if result.returncode == 0 else f"Error running ockam: {result.stderr}"
    except Exception as e:
        return f"Error: {str(e)} (CWD: {os.getcwd()})"


class handler(BaseHTTPRequestHandler):
    """Handler for PostgreSQL database requests via Ockam secure channel."""

    def create_ockam_node(self, enrollment_ticket: str) -> bool:
        """Initialize Ockam node with the provided enrollment ticket."""
        if not is_production():
            return False

        global NODE_INITIALIZED
        request_id = self.headers.get('x-vercel-id', os.urandom(8).hex())

        print(f"[{request_id}] Starting with NODE_INITIALIZED = {NODE_INITIALIZED}")

        if not NODE_INITIALIZED:
            NODE_INITIALIZED = True

            def run_ockam_node():
                try:
                    config = '''{
                        "tcp-inlet": {
                            "from": "127.0.0.1:15432",
                            "via": "postgresql",
                            "allow": "amazon-rds-postgresql-outlet"
                        }
                    }'''

                    print(f"[{request_id}] Starting ockam node with config: {config}", flush=True)

                    subprocess.Popen([
                        OCKAM_BINARY_PATH,
                        'node',
                        'create',
                        '--configuration',
                        config,
                        '--enrollment-ticket',
                        enrollment_ticket.strip()
                    ], env={
                        **os.environ,
                        'OCKAM_HOME': '/tmp',
                        'OCKAM_OPENTELEMETRY_EXPORT': 'false',
                        'OCKAM_DISABLE_UPGRADE_CHECK': 'true'
                    })

                except Exception as e:
                    print(f"[{request_id}] Error in node creation: {str(e)}", flush=True)

            thread = threading.Thread(target=run_ockam_node)
            thread.daemon = True
            print(f"[{request_id}] Starting ockam node thread", flush=True)
            thread.start()

        return True

    def get_db_connection(self, request_id: str):
        """Establish database connection with retries."""
        db_password = os.environ.get('DB_PASSWORD')
        if not db_password:
            raise ValueError("DB_PASSWORD environment variable is not set")

        for retry in range(MAX_RETRIES):
            try:
                conn = psycopg2.connect(
                    host="127.0.0.1",
                    port=DB_PORT,
                    database="testdb",
                    user="postgresuser",
                    password=db_password,
                    connect_timeout=DB_TIMEOUT
                )
                print(f"[{request_id}] Connected to PostgreSQL successfully after {retry} retries")
                conn.set_isolation_level(ISOLATION_LEVEL_AUTOCOMMIT)
                return conn, None

            except Exception as e:
                print(f"[{request_id}] Attempt {retry + 1} failed: {str(e)}")
                if retry < MAX_RETRIES - 1:
                    time.sleep(RETRY_DELAY)

        return None, f"Failed to connect after {MAX_RETRIES} attempts"

    def handle_select(self, request_id: str) -> dict:
        """Handle GET requests for product data."""
        conn, error = self.get_db_connection(request_id)
        if error:
            return {"status": "error", "message": f"Database connection failed: {error}"}

        try:
            cur = conn.cursor()
            cur.execute("SELECT * FROM products;")
            rows = cur.fetchall()
            formatted_rows = [
                {
                    'id': row[0],
                    'product_name': row[1],
                    'price': round(float(row[2]), 2) if isinstance(row[2], Decimal) else row[2]
                }
                for row in rows
            ]

            cur.close()
            conn.close()

            return {"status": "success", "data": formatted_rows}
        except Exception as e:
            return {"status": "error", "message": str(e)}

    def handle_update(self, request_id: str) -> dict:
        """Handle requests to update product prices."""
        conn, error = self.get_db_connection(request_id)
        if error:
            return {"status": "error", "message": f"Database connection failed: {error}"}

        try:
            cur = conn.cursor()
            cur.execute("UPDATE products SET price = (random() * 90 + 10)::numeric(10,2);")

            # Get updated data
            cur.execute("SELECT * FROM products;")
            rows = cur.fetchall()
            formatted_rows = [
                {
                    'id': row[0],
                    'product_name': row[1],
                    'price': round(float(row[2]), 2) if isinstance(row[2], Decimal) else row[2]
                }
                for row in rows
            ]

            cur.close()
            conn.close()

            return {
                "status": "success",
                "message": "Prices updated successfully",
                "data": formatted_rows
            }
        except Exception as e:
            return {"status": "error", "message": str(e)}

    def do_GET(self):
        """Handle incoming GET requests."""
        try:
            # Verify Ockam installation
            version_result = get_ockam_version()
            if 'Error' in version_result:
                self._send_error(500, version_result)
                return

            # Verify enrollment ticket
            enrollment_ticket = os.environ.get('OCKAM_RDS_INLET_ENROLLMENT_TICKET')
            if not enrollment_ticket:
                self._send_error(500, 'OCKAM_RDS_INLET_ENROLLMENT_TICKET not configured')
                return

            # Initialize Ockam node
            if not self.create_ockam_node(enrollment_ticket):
                self._send_error(500, 'Failed to initialize Ockam node')
                return

            # Handle the request
            request_id = self.headers.get('x-vercel-id', os.urandom(8).hex())
            result = self.handle_update(request_id) if self.path == '/api/update' else self.handle_select(request_id)

            self._send_response(200, result)

        except Exception as e:
            self._send_error(500, str(e))

    def _send_response(self, status: int, data: dict):
        """Helper method to send JSON responses."""
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps(data).encode())

    def _send_error(self, status: int, message: str):
        """Helper method to send error responses."""
        self._send_response(status, {'error': message})
