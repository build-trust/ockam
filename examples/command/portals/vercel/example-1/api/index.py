"""
Simple API handler for accessing Snowflake private API endpoints via Ockam portal
"""

import json
import os
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler

import requests

# Constants
OCKAM_BINARY_PATH = '/var/task/data/linux-x86_64/ockam' if os.environ.get('VERCEL_ENV') == 'production' else 'ockam'
MAX_RETRIES = 60
RETRY_DELAY = 1  # seconds
LOCAL_API_ENDPOINT = "http://127.0.0.1:8081"
REQUEST_TIMEOUT = 3  # seconds

# Track if Ockam node creation has been attempted
# Only one Ockam node should run per serverless function runtime - this flag prevents multiple node creation attempts.
NODE_INITIALIZED = False


def is_production() -> bool:
    """Check if the environment is production."""
    return os.environ.get('VERCEL_ENV') == 'production'


def get_ockam_version() -> str:
    """
    Get the installed Ockam version.
    Returns version string or error message.
    """
    try:
        ockam_path = OCKAM_BINARY_PATH
        if is_production() and not os.path.exists(ockam_path):
            return f"Error: Ockam binary not found at {os.path.abspath(ockam_path)}"

        result = subprocess.run([ockam_path, '--version'], capture_output=True, text=True)

        return result.stdout.strip() if result.returncode == 0 else f"Error running ockam: {result.stderr}"
    except Exception as e:
        return f"Error: {str(e)} (CWD: {os.getcwd()})"


class handler(BaseHTTPRequestHandler):
    """Handler for Snowflake API requests via Ockam secure channel."""

    def create_ockam_node(self, enrollment_ticket: str) -> bool:
        """
        Initialize Ockam node with the provided enrollment ticket.
        Returns True if successful or already attempted, False otherwise.
        """
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
                            "from": "127.0.0.1:8081",
                            "via": "snowflake-api-service-relay",
                            "allow": "snowflake-api-service-outlet"
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
                        'OCKAM_TELEMETRY_EXPORT': 'false',
                        'OCKAM_DISABLE_UPGRADE_CHECK': 'true'
                    })

                except Exception as e:
                    print(f"[{request_id}] Error in node creation: {str(e)}", flush=True)

            thread = threading.Thread(target=run_ockam_node)
            thread.daemon = True
            print(f"[{request_id}] Starting ockam node thread", flush=True)
            thread.start()

        return True

    def handle_select(self, request_id: str) -> dict:
        """Handle GET requests for product data."""
        for retry in range(MAX_RETRIES):
            try:
                response = requests.get(
                    f"{LOCAL_API_ENDPOINT}/connector/products",
                    timeout=REQUEST_TIMEOUT
                )
                response.raise_for_status()
                print(f"[{request_id}] Connected successfully after {retry} retries")
                return {"status": "success", "data": response.json()}

            except Exception as e:
                print(f"[{request_id}] Attempt {retry + 1} failed: {str(e)}")
                if retry < MAX_RETRIES - 1:
                    time.sleep(RETRY_DELAY)
                else:
                    return {"status": "error", "message": f"Connection failed: {str(e)}"}

    def handle_update(self, request_id: str) -> dict:
        """Handle POST requests for product updates."""
        for retry in range(MAX_RETRIES):
            try:
                response = requests.post(
                    f"{LOCAL_API_ENDPOINT}/connector/products/update",
                    timeout=REQUEST_TIMEOUT
                )
                response.raise_for_status()
                print(f"[{request_id}] Update successful after {retry} retries")

                updated_values = self.handle_select(request_id)
                return {
                    "status": "success",
                    "update_result": response.json(),
                    "current_values": updated_values["data"]
                }

            except Exception as e:
                print(f"[{request_id}] Attempt {retry + 1} failed: {str(e)}")
                if retry < MAX_RETRIES - 1:
                    time.sleep(RETRY_DELAY)
                else:
                    return {"status": "error", "message": f"Update failed: {str(e)}"}

    def do_GET(self):
        """Handle incoming GET requests."""
        try:
            # Verify Ockam installation
            version_result = get_ockam_version()
            if 'Error' in version_result:
                self._send_error(500, version_result)
                return

            # Verify enrollment ticket
            enrollment_ticket = os.environ.get('OCKAM_SNOWFLAKE_INLET_ENROLLMENT_TICKET')
            if not enrollment_ticket:
                self._send_error(500, 'OCKAM_SNOWFLAKE_INLET_ENROLLMENT_TICKET not configured')
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
