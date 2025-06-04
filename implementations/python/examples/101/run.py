import os
import signal
import subprocess
import sys
import time


box_mock_handle = None
main_handle = None
runners = []


def main():
    global box_mock_handle, main_handle, runners

    import json

    with open("config.json", "r") as f:
        config = json.load(f)

    box_node = config["box_node"]
    main_node = config["main_node"]
    runner_nodes = config["runner_nodes"]

    num_of_runners = len(runner_nodes)

    env = os.environ.copy()
    env["CLUSTER"] = "acme"
    env["ZONE"] = "squad"
    env["OCKAM_SQLITE_IN_MEMORY"] = "1"

    derived_env = env.copy()
    derived_env["ENROLLMENT_TICKET"] = box_node["ticket"]
    derived_env["NODE"] = box_node["name"]
    box_mock_handle = subprocess.Popen(["python3", "main.py"], env=derived_env, cwd="./box")

    derived_env = env.copy()
    derived_env["ENROLLMENT_TICKET"] = main_node["ticket"]
    derived_env["NODE"] = main_node["name"]
    main_handle = subprocess.Popen(
        ["python3", "main.py", "127.0.0.1:9000"],
        env=derived_env,
        cwd="./main",
    )

    for i in range(num_of_runners):
        derived_env = env.copy()
        derived_env["ENROLLMENT_TICKET"] = runner_nodes[i]["ticket"]
        derived_env["NODE"] = runner_nodes[i]["name"]
        handle = subprocess.Popen(
            ["python3", "main.py", f"127.0.0.1:{9001 + i}"],
            env=derived_env,
            cwd="./runner",
        )
        runners.append(handle)

    while True:
        time.sleep(1)


def handle_sigint(signum, frame):
    print("\nSIGINT received. Cleaning up...")

    if box_mock_handle:
        box_mock_handle.terminate()
        box_mock_handle.wait()

    if main_handle:
        main_handle.terminate()
        main_handle.wait()

    for runner in runners:
        runner.terminate()
        runner.wait()

    sys.exit(0)


signal.signal(signal.SIGINT, handle_sigint)


if __name__ == "__main__":
    main()
