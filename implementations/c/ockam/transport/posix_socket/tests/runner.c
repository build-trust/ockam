#include <stdio.h>
#include <unistd.h>
#include <sys/wait.h>

#include "ockam/error.h"
#include "ockam/memory.h"
#include "ockam/log/syslog.h"
#include "ockam/transport.h"
#include "tools.h"
#include "client.h"
#include "server.h"
#include "runner.h"

int run(enum TransportType transport_type, int argc, char* argv[])
{
  test_cli_params_t test_params;

  int32_t test_server_process = 0;
  int     test_client_error   = 0;
  int     test_server_error   = 0;


  ockam_error_t error = OCKAM_ERROR_NONE;

  error = init_params(transport_type, argc, argv, &test_params);
  if (error) {
    goto exit;
  }

  if (test_params.run_server) {
    printf("Run Server\n");
    test_server_process = fork();
    if (test_server_process < 0) {
      log_error(TRANSPORT_ERROR_TEST, "Fork unsuccessful");
      error = -1;
      goto exit;
    }
  }
  if ((0 != test_server_process) || !test_params.run_server) {
    if (test_params.run_client) {
      printf("Run Client\n");
      error = run_test_client(&test_params);
      if (0 != error) {
        log_error(TRANSPORT_ERROR_TEST, "testTcpClient failed");
        test_client_error = -1;
      }
    }
    // Get exit error from testServerProcess
    if (test_params.run_server) {
      int fork_error = 0;
      wait(&fork_error);
      test_server_error = WEXITSTATUS(fork_error);
      if (0 != test_server_error) { test_server_error = -2; }
      error = test_server_error + test_client_error;
      if (!error) printf("Transport test successful!\n");
    }
  } else if (test_params.run_server) {
    // This is the server process
    error = run_test_server(&test_params);
    if (0 != error) {
      log_error(TRANSPORT_ERROR_TEST, "testTcpServer failed");
      error = -1;
    }
  }

exit:
  if (error) {
    log_error(error, "Error during transport test run");
    return error;
  }

  return 0;
}