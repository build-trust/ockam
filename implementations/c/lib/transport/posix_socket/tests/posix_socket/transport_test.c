#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <unistd.h>
#include <sys/wait.h>

#include "ockam/error.h"
#include "ockam/memory.h"
#include "ockam/syslog.h"
#include "ockam/transport.h"
#include "tests.h"

#define DEFAULT_FIXTURE_PATH "fixtures"
#define DEFAULT_IP_ADDRESS   "127.0.0.1"
#define DEFAULT_LISTEN_PORT  8000
#define FIXTURE_PATH_LEN     192

bool run_client   = false;
bool run_server   = false;
bool run_tcp_test = 0;
bool run_udp_test = 0;

int test_client(ockam_ip_address_t* address, char* p_fixture_path);
int test_server(ockam_ip_address_t* address, char* p_fixture_path);

void usage()
{
  printf("OPTIONS\n");
  printf("  -a:<xxx.xxx.xxx.xxx>\t\tIP Address\n");
  printf("  -p:<portnum>\t\t\tPort\n");
  printf("  -c \t\t\t\tRun client\n");
  printf("  -s \t\t\t\tRun server\n");
  printf("  -u \t\t\t\tRun UDP test\n");
  printf("  -t \t\t\t\tRun TCP test\n");
}

ockam_error_t parse_opts(int argc, char* argv[], ockam_ip_address_t* p_address, char* p_fixture_path)
{
  int           ch;
  ockam_error_t status = OCKAM_ERROR_NONE;
  p_address->port      = DEFAULT_LISTEN_PORT;
  strcpy((char*) p_address->ip_address, DEFAULT_IP_ADDRESS);
  while ((ch = getopt(argc, argv, "tuha:p:csf:?")) != -1) {
    switch (ch) {
    case 'a':
      strcpy((char*) p_address->ip_address, optarg);
      break;

    case 'p':
      p_address->port = strtoul(optarg, NULL, 0);
      break;

    case 'c':
      run_client = true;
      break;

    case 's':
      run_server = true;
      break;

    case 'u':
      run_udp_test = 1;
      break;

    case 't':
      run_tcp_test = 1;
      break;

    case 'f':
      printf("optarg: %s\n", optarg);
      strncpy(p_fixture_path, optarg, FIXTURE_PATH_LEN);
      break;

    case 'h':

    case '?':
      status = TRANSPORT_ERROR_BAD_PARAMETER;
      usage();
      log_error(status, "invalid command-line arguments");
      return 2;

    default:
      break;
    }
  }

  if (run_tcp_test && run_udp_test) {
    printf("Can't run both UDP and TCP tests, pick one or the other\n");
    usage();
    return 2;
  }
  if (!(run_tcp_test || run_udp_test)) {
    printf("Select either UDP or TCP test\n");
    usage();
    return 2;
  }

  return status;
}

int main(int argc, char* argv[])
{
  ockam_error_t      error                          = 0;
  int                test_server_error              = 0;
  int                test_client_error              = 0;
  int                fork_error                     = 0;
  int32_t            test_server_process            = 0;
  char               fixture_path[FIXTURE_PATH_LEN] = { 0 };
  ockam_ip_address_t ip_address;

  ip_address.port = DEFAULT_LISTEN_PORT;
  strcpy((char*) &(ip_address.ip_address)[0], DEFAULT_IP_ADDRESS);
  strcpy(fixture_path, DEFAULT_FIXTURE_PATH);

  parse_opts(argc, argv, &ip_address, fixture_path);

  // error = test_client(&ip_address, &fixture_path[0], run_protocol);
  // error = test_server(&ip_address, &fixture_path[0], run_protocol);
  // goto exit;

  if (run_server) {
    printf("Run Server\n");
    test_server_process = fork();
    if (test_server_process < 0) {
      log_error(TRANSPORT_ERROR_TEST, "Fork unsuccessful");
      error = -1;
      goto exit;
    }
  }
  if ((0 != test_server_process) || !run_server) {
    if (run_client) {
      printf("Run Client\n");
      error = test_client(&ip_address, &fixture_path[0]);
      if (0 != error) {
        log_error(TRANSPORT_ERROR_TEST, "testTcpClient failed");
        test_client_error = -1;
      }
    }
    // Get exit error from testServerProcess
    if (run_server) {
      wait(&fork_error);
      test_server_error = WEXITSTATUS(fork_error);
      if (0 != test_server_error) { test_server_error = -2; }
      error = test_server_error + test_client_error;
      if (!error) printf("Transport test successful!\n");
    }
  } else if (run_server) {
    // This is the server process
    error = test_server(&ip_address, &fixture_path[0]);
    if (0 != error) {
      log_error(TRANSPORT_ERROR_TEST, "testTcpServer failed");
      error = -1;
    }
  }

exit:
  return error;
}
