#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <unistd.h>
#include <sys/wait.h>
#include <getopt.h>

#include "ockam/error.h"
#include "ockam/key_agreement.h"
#include "ockam/memory.h"
#include "ockam/memory/stdlib.h"
#include "ockam/log.h"
#include "ockam/transport.h"

#include "ockam/random/urandom.h"
#include "ockam/vault/default.h"

#include "init_vault.h"

#include "xx_test.h"

#define ACK_TEXT "ACK_TEXT"
#define ACK_SIZE 3
#define OK       "OK"
#define OK_SIZE  2

bool               scripted_xx        = false;
// FIXME: CI tests will run the test without enabling initiator&responder
bool               run_initiator      = true;
bool               run_responder      = true;
uint8_t            vault_opt   = VAULT_OPT_DEFAULT;
ockam_ip_address_t ockam_initiator_ip = { "", "127.0.0.1", 4050 };
ockam_ip_address_t ockam_responder_ip = { "", "127.0.0.1", 4051 };

void usage()
{
  printf("OPTIONS\n");
  printf("  -a<xxx.xxx.xxx.xxx:xxxx>\t\tInitiator IP address & port\n");
  printf("  -b<xxx.xxx.xxx.xxx:xxxx>\t\tResponder IP address & port");
  printf("  --no-client \t\tDo not run initiator\n");
  printf("  --no-server \t\tDo not run responder\n");
  printf("  -s \t\t\t\tUse scripted test case\n\n");
  printf("  -v<1:2> \t\t\t\tVault: 1 - Default, 2 - ATECC608A\n\n");
}

int parse_opts(int argc, char* argv[])
{
  static int no_client = 0;
  static int no_server = 0;

  static struct option long_options[] = {
          /* These options set a flag. */
          {"no-client",   no_argument,       &no_client, 1},
          {"no-server",   no_argument,       &no_server, 1},
          {0, 0, 0, 0}
  };

  int option_index = 0;
  int ch;
  int status = 0;
  while ((ch = getopt_long(argc, argv, "hsa:b:v:", long_options, &option_index)) != -1) {
    switch (ch) {
    case 'h':
      usage();
      return 2;

    case 'a': {
      char*    token = NULL;
      uint16_t port  = 0;
      token          = strtok(optarg, ":");
      strcpy((char*) ockam_initiator_ip.ip_address, token);
      token                   = strtok(NULL, ":");
      ockam_initiator_ip.port = atoi(token);
      break;
    }

    case 'b': {
      char*    token = NULL;
      uint16_t port  = 0;
      token          = strtok(optarg, ":");
      strcpy((char*) ockam_responder_ip.ip_address, token);
      token                   = strtok(NULL, ":");
      ockam_responder_ip.port = atoi(token);
      break;
    }

    case 's':
      scripted_xx = true;
      break;

    case 'v':
      vault_opt = atoi(optarg);
      break;

    case '?':
      status = -1;
      usage();
      ockam_log_error("invalid command-line arguments: %d", status);
      return 2;

    default:
      break;
    }
  }

  run_initiator = no_client == 0;
  run_responder = no_server == 0;

  return status;
}

int main(int argc, char* argv[])
{
  int status = parse_opts(argc, argv);
  if (status) goto exit;

  ockam_log_info("Initiator     : %s:%u", ockam_initiator_ip.ip_address, ockam_initiator_ip.port);
  ockam_log_info("Responder     : %s:%u", ockam_responder_ip.ip_address, ockam_responder_ip.port);
  ockam_log_info("Run initiator : %d", run_initiator);
  ockam_log_info("Run responder : %d", run_responder);
  ockam_log_info("Vault         : %d", vault_opt);
  ockam_log_info("Run script    : %d", scripted_xx);

  ockam_vault_t  vault             = { 0 };
  ockam_memory_t memory            = { 0 };
  ockam_random_t random            = { 0 };

  ockam_error_t error = ockam_memory_stdlib_init(&memory);
  if (ockam_error_has_error(&error)) goto exit;
    ockam_log_info("Memory init success");

  error = ockam_random_urandom_init(&random);
  if (ockam_error_has_error(&error)) goto exit;
    ockam_log_info("Random init success");

  error = init_vault(&vault, vault_opt, &memory, &random);
  if (ockam_error_has_error(&error)) goto exit;
    ockam_log_info("Vault initiator init success");

  bool require_fork = run_initiator && run_responder;

  bool is_child = false;
  if (require_fork) {
      int32_t responder_process = fork();
      if (responder_process < 0) {
          error.code = -1;
          goto exit;
      }
      is_child = 0 == responder_process;
  }

    if (run_initiator && (is_child || !require_fork)) {
        ockam_log_info("Starting initiator");
      error = xx_test_initiator(&vault, &memory, &ockam_initiator_ip, &ockam_responder_ip);
      if (ockam_error_has_error(&error)) {
        goto exit;
      }
        ockam_log_info("Initiator finished successfully");
    }
    if (run_responder && (!is_child)) {
        ockam_log_info("Starting responder");
      // This is the server process
      error = xx_test_responder(&vault, &memory, &ockam_responder_ip);
      if (ockam_error_has_error(&error)) goto exit;
        ockam_log_info("Initiator finished successfully");
    }

    if (require_fork && !is_child) {
        // Get exit status from responder_process
        int            fork_status       = 0;
        wait(&fork_status);
        int32_t responder_status = WEXITSTATUS(fork_status);
        if (responder_status) {
            error.code = -1;
            goto exit;
        }
    }

exit:
  printf("Tests done\n");
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  if (status) ockam_log_error("Status: %d", status);

  return error.code + status;
}
