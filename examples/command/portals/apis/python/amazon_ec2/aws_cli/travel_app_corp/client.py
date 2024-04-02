import requests
import time


def run(attempts=20, timeout=1, time_between_attempts=2):
    for i in range(attempts):
        try:
            res = requests.get('http://localhost:5000/ping', timeout=timeout)
            if res.status_code == 200:
                print(res.text)
                print(
                    "\nThe example run was successful 🥳.\n" +
                    "The app made an API request to the monitoring API over an encrypted portal and got back a response.\n",
                )
                return

        except requests.exceptions.ConnectionError:
            print("Connection error")
            if i < attempts - 1:
                print("Waiting for " + str(time_between_attempts) +
                      " seconds before next attempt...")
                time.sleep(time_between_attempts)
            pass

    print("All attempts failed")


run()
