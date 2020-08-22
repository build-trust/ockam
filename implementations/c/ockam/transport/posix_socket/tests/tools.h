#ifndef OCKAM_TOOLS_H
#define OCKAM_TOOLS_H

#include <stdbool.h>
#include <stdio.h>
#include <ockam/memory.h>
#include <ockam/memory/impl.h>
#include "runner.h"

#define FIXTURE_PATH_MAX_LEN 192
#define ENDING_LINE          "that's all"

typedef struct {
  bool               run_client;
  bool               run_server;
  bool               run_udp_test;
  bool               run_tcp_test;
  ockam_ip_address_t client_address;
  ockam_ip_address_t server_address;
  char               fixture_path[FIXTURE_PATH_MAX_LEN];
  ockam_memory_t     memory;
} test_cli_params_t;

ockam_error_t init_params(enum TransportType transport_type, int argc, char* argv[], test_cli_params_t* p_params);

ockam_error_t open_file_for_client_send(const char* p_fixture_path, FILE** pp_file);
ockam_error_t open_file_for_client_receive(const char* p_fixture_path, FILE** pp_file);
ockam_error_t open_files_for_client_compare(const char* p_fixture_path, FILE** pp_sent_file, FILE** pp_received_file);
ockam_error_t open_files_for_server_send(const char* p_fixture_path, FILE** pp_file);
ockam_error_t open_files_for_server_receive(const char* p_fixture_path, FILE** pp_file);
ockam_error_t open_files_for_server_compare(const char* p_fixture_path, FILE** pp_sent_file, FILE** pp_received_file);

ockam_error_t file_compare(ockam_memory_t* p_memory, FILE* p_f1, FILE* p_f2);

#endif // OCKAM_TOOLS_H
