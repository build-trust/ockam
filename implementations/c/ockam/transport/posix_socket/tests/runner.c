#include <stdio.h>
#include <unistd.h>
#include <sys/wait.h>

#include "ockam/error.h"
#include "ockam/memory.h"
#include "ockam/log.h"
#include "ockam/transport.h"
#include "tools.h"
#include "client.h"
#include "server.h"
#include "runner.h"

int run(enum TransportType transport_type, int argc, char* argv[])
{
  ockam_log_info("Transport test runner started");

  test_cli_params_t test_params;

  int32_t test_server_process = 0;
  int     test_client_error   = 0;
  int     test_server_error   = 0;


  ockam_error_t error = OCKAM_ERROR_NONE;

  error = init_params(transport_type, argc, argv, &test_params);
  if (error) {
    goto exit;
  }

  bool is_parent = true;
  if (test_params.run_server) {
    ockam_log_info("Starting fork");
    test_server_process = fork();
    if (test_server_process < 0) {
      ockam_log_error("%s", "Fork unsuccessful");
      error = -1;
      goto exit;
    }
    is_parent = (test_server_process != 0);
  }
  if (is_parent || !test_params.run_server) {
    if (test_params.run_client) {
      ockam_log_info("Starting client");
      error = run_test_client(&test_params);
      ockam_log_info("Client finished");
      if (0 != error) {
        ockam_log_error("%s", "testTcpClient failed");
        test_client_error = -1;
      }
    }
    // Get exit error from testServerProcess
    if (test_params.run_server) {
      ockam_log_info("Waiting for fork to finish");
      int fork_error = 0;
      wait(&fork_error);
      test_server_error = WEXITSTATUS(fork_error);
      ockam_log_info("Fork finished");
      if (0 != test_server_error) { test_server_error = -2; }
      error = test_server_error + test_client_error;
      if (!error) ockam_log_info("Transport test successful!");
    }
  } else if (test_params.run_server) {
    ockam_log_info("Starting server");
    error = run_test_server(&test_params);
    ockam_log_info("Server finished");
    if (0 != error) {
      ockam_log_error("%s", "testTcpServer failed");
      error = -1;
    }
  }

exit:
  if (error) {
    ockam_log_error("%s", "Error during transport test run");
    return error;
  }

  return 0;
}