#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <stdbool.h>
#include <ockam/memory/stdlib.h>
#include <getopt.h>
#include <ockam/transport/socket.h>

#include "ockam/error.h"
#include "ockam/log.h"
#include "ockam/transport.h"

#include "tools.h"

#define DEFAULT_FIXTURE_PATH      "fixtures"
#define DEFAULT_SERVER_IP_ADDRESS "127.0.0.1"
#define DEFAULT_CLIENT_IP_ADDRESS "127.0.0.1"
#define DEFAULT_SERVER_PORT       8000
#define DEFAULT_CLIENT_PORT       8002

static void print_usage()
{
  printf("OPTIONS\n");
  printf("  --server-ip:<xxx.xxx.xxx.xxx>\t\tServer IP Address\n");
  printf("  --client-ip:<xxx.xxx.xxx.xxx>\t\tClient IP Address\n");
  printf("  --server-port:<portnum>\t\t\tServer port\n");
  printf("  --client-port:<portnum>\t\t\tClient port\n");
  printf("  --no-client \t\tDo not run client\n");
  printf("  --no-server \t\tDo not run server\n");
  printf("  -f:<path>\t\t\tFixture path\n");
}

ockam_error_t init_params(enum TransportType transport_type, int argc, char* argv[], test_cli_params_t* p_params)
{
  ockam_error_t status = ockam_transport_posix_socket_error_none;
  if (NULL == p_params) {
    status.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_PARAMETER;
    goto exit;
  }

  switch (transport_type) {
  case TCP:
    p_params->run_tcp_test = true;
    p_params->run_udp_test = false;
    break;

  case UDP:
    p_params->run_tcp_test = false;
    p_params->run_udp_test = true;
    break;

  default:
    status.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_PARAMETER;
    goto exit;
  }

  status = ockam_memory_stdlib_init(&p_params->memory);
  if (ockam_error_has_error(&status)) { goto exit; }

  strcpy(p_params->fixture_path, DEFAULT_FIXTURE_PATH);

  p_params->server_address.port = DEFAULT_SERVER_PORT;
  strcpy((char*) p_params->server_address.ip_address, DEFAULT_SERVER_IP_ADDRESS);

  p_params->client_address.port = DEFAULT_CLIENT_PORT; /* Not used for tcp */
  strcpy((char*) p_params->client_address.ip_address, DEFAULT_CLIENT_IP_ADDRESS);

  static int no_client = 0;
  static int no_server = 0;

  static struct option long_options[] = { /* These options set a flag. */
                                          { "no-client", no_argument, &no_client, 1 },
                                          { "no-server", no_argument, &no_server, 1 },
                                          { "server-ip", required_argument, NULL, 1 },
                                          { "server-port", required_argument, NULL, 2 },
                                          { "client-ip", required_argument, NULL, 3 },
                                          { "client-port", required_argument, NULL, 4 },
                                          { 0, 0, 0, 0 }
  };

  int option_index = 0;
  int ch;

  while ((ch = getopt_long(argc, argv, "ha:p:f:?", long_options, &option_index)) != -1) {
    switch (ch) {
    case 0:
      break;

    case 1:
      // TODO: validate optarg value somehow?
      strcpy((char*) p_params->server_address.ip_address, optarg);
      break;

    case 2:
      p_params->server_address.port = strtoul(optarg, NULL, 0);
      break;

    case 3:
      strcpy((char*) p_params->client_address.ip_address, optarg);
      break;

    case 4:
      p_params->client_address.port = strtoul(optarg, NULL, 0);
      break;

    case 'f':
      printf("optarg: %s\n", optarg);
      strncpy(p_params->fixture_path, optarg, FIXTURE_PATH_MAX_LEN);
      break;

    default:
      status.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_PARAMETER;
      print_usage();
      ockam_log_fatal("Bad parameter");
      goto exit;
    }
  }

  p_params->run_client = no_client == 0;
  p_params->run_server = no_server == 0;

exit:
  return status;
}

static const char CLIENT_TEST_DATA[]     = "client_test_data.txt";
static const char SERVER_TEST_DATA[]     = "server_test_data.txt";
static const char SERVER_RECEIVED_DATA[] = "server_data_received.txt";
static const char CLIENT_RECEIVED_DATA[] = "client_data_received.txt";

static void make_file_path(const char* p_fixture_path, const char* p_file_name, char* p_path)
{
  sprintf(p_path, "%s/%s", p_fixture_path, p_file_name);
}

ockam_error_t open_file_for_read(const char* p_fixture_path, const char* p_file_name, FILE** pp_file)
{
  ockam_error_t error = ockam_transport_posix_socket_error_none;

  char path[256];

  make_file_path(p_fixture_path, p_file_name, path);
  *pp_file = fopen(path, "r");
  if (NULL == *pp_file) {
    error.code = -1;
    ockam_log_fatal("%s", "failed to open file");
    goto exit;
  }

exit:
  return error;
}

ockam_error_t open_file_for_write(const char* p_fixture_path, const char* p_file_name, FILE** pp_file)
{
  ockam_error_t error = ockam_transport_posix_socket_error_none;

  char path[256];

  make_file_path(p_fixture_path, p_file_name, path);
  *pp_file = fopen(path, "w");
  if (NULL == *pp_file) {
    error.code = -1;
    ockam_log_fatal("%s", "failed to open file");
    goto exit;
  }

exit:
  return error;
}

ockam_error_t open_file_for_client_send(const char* p_fixture_path, FILE** pp_file)
{
  return open_file_for_read(p_fixture_path, CLIENT_TEST_DATA, pp_file);
}

ockam_error_t open_file_for_client_receive(const char* p_fixture_path, FILE** pp_file)
{
  return open_file_for_write(p_fixture_path, CLIENT_RECEIVED_DATA, pp_file);
}

ockam_error_t open_files_for_client_compare(const char* p_fixture_path, FILE** pp_sent_file, FILE** pp_received_file)
{
  ockam_error_t error = ockam_transport_posix_socket_error_none;

  error = open_file_for_read(p_fixture_path, SERVER_TEST_DATA, pp_sent_file);
  if (ockam_error_has_error(&error)) goto exit;

  error = open_file_for_read(p_fixture_path, CLIENT_RECEIVED_DATA, pp_received_file);
  if (ockam_error_has_error(&error)) goto exit;

exit:
  return error;
}

ockam_error_t open_files_for_server_send(const char* p_fixture_path, FILE** pp_file)
{
  return open_file_for_read(p_fixture_path, SERVER_TEST_DATA, pp_file);
}

ockam_error_t open_files_for_server_receive(const char* p_fixture_path, FILE** pp_file)
{
  return open_file_for_write(p_fixture_path, SERVER_RECEIVED_DATA, pp_file);
}

ockam_error_t open_files_for_server_compare(const char* p_fixture_path, FILE** pp_sent_file, FILE** pp_received_file)
{
  ockam_error_t error = ockam_transport_posix_socket_error_none;

  error = open_file_for_read(p_fixture_path, CLIENT_TEST_DATA, pp_sent_file);
  if (ockam_error_has_error(&error)) goto exit;

  error = open_file_for_read(p_fixture_path, SERVER_RECEIVED_DATA, pp_received_file);
  if (ockam_error_has_error(&error)) goto exit;

exit:
  return error;
}

ockam_error_t file_compare(ockam_memory_t* p_memory, FILE* p_f1, FILE* p_f2)
{
  ockam_error_t status = ockam_transport_posix_socket_error_none;

  char buffer1[256];
  char buffer2[256];

  size_t r1;
  size_t r2;

  if ((NULL == p_f1) || (NULL == p_f2)) {
    status.code = -1;
    goto exit_block;
  }

  while (true) {
    r1 = fread(buffer1, 1, sizeof(buffer1), p_f1);
    r2 = fread(buffer2, 1, sizeof(buffer2), p_f2);
    if (r1 != r2) {
      status.code = -1;
      goto exit_block;
    }
    int cmp = 2;
    status = ockam_memory_compare(p_memory, &cmp, buffer1, buffer2, r1);
    if (ockam_error_has_error(&status)) {
      goto exit_block;
    }
    if (0 != cmp) {
      status.code = -1;
      goto exit_block;
    }
    if (feof(p_f1) || feof(p_f2)) {
      if (feof(p_f1) != feof(p_f2)) {
        status.code = -1;
        goto exit_block;
      }

      break;
    }
  }

exit_block:
  return status;
}
