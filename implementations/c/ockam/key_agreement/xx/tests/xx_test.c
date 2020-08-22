
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <unistd.h>
#include <sys/wait.h>

#include "ockam/error.h"
#include "ockam/key_agreement.h"
#include "ockam/memory.h"
#include "ockam/memory/stdlib.h"
#include "ockam/log.h"
#include "ockam/transport.h"

#include "ockam/random/urandom.h"
#include "ockam/vault/default.h"

#include "xx_test.h"

#define ACK_TEXT "ACK_TEXT"
#define ACK_SIZE 3
#define OK       "OK"
#define OK_SIZE  2

bool               scripted_xx        = false;
bool               run_initiator      = false;
bool               run_responder      = false;
ockam_ip_address_t ockam_initiator_ip = { "", "127.0.0.1", 4050 };
ockam_ip_address_t ockam_responder_ip = { "", "127.0.0.1", 4051 };

void usage()
{
  printf("OPTIONS\n");
  printf("  -a<xxx.xxx.xxx.xxx:xxxx>\t\tInitiator IP address & port\n");
  printf("  -b<xxx.xxx.xxx.xxx:xxxx>\t\tResponder IP address & port");
  printf("  -i \t\t\t\tRun initiator\n");
  printf("  -r \t\t\t\tRun responder\n");
  printf("  -s \t\t\t\tUse scripted test case\n\n");
}

ockam_error_t parse_opts(int argc, char* argv[])
{
  int           ch;
  ockam_error_t status = OCKAM_ERROR_NONE;
  while ((ch = getopt(argc, argv, "hsira:b:")) != -1) {
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

    case 'i':
      run_initiator = true;
      break;

    case 'r':
      run_responder = true;
      break;

    case 's':
      scripted_xx = true;
      break;

    case '?':
      status = TRANSPORT_ERROR_BAD_PARAMETER;
      usage();
      ockam_log_error("%x", TRANSPORT_ERROR_BAD_PARAMETER);
      return 2;

    default:
      break;
    }
  }

  return status;
}

int main(int argc, char* argv[])
{
  ockam_error_t  error             = OCKAM_ERROR_NONE;
  ockam_vault_t  vault             = { 0 };
  ockam_memory_t memory            = { 0 };
  ockam_random_t random            = { 0 };
  int            responder_status  = 0;
  int            initiator_status  = 0;
  int            fork_status       = 0;
  int32_t        responder_process = 0;

  ockam_vault_default_attributes_t vault_attributes = { .memory = &memory, .random = &random };

  error = ockam_memory_stdlib_init(&memory);
  if (error) goto exit;

  error = ockam_random_urandom_init(&random);
  if (error) goto exit;

  error = ockam_vault_default_init(&vault, &vault_attributes);
  if (error) goto exit;

  ockam_log_set_level(OCKAM_LOG_LEVEL_ERROR);

  /*-------------------------------------------------------------------------
   * Parse options
   *-----------------------------------------------------------------------*/
  error = parse_opts(argc, argv);
  if (error) goto exit;
  printf("Initiator       : %s:%u\n", ockam_initiator_ip.ip_address, ockam_initiator_ip.port);
  printf("Responder       : %s:%u\n", ockam_responder_ip.ip_address, ockam_responder_ip.port);
  printf("Run initiator   : %d\n", run_initiator);
  printf("Run responder   : %d\n", run_responder);
  printf("Run script      : %d\n", scripted_xx);

  // error = xx_test_responder(&vault, &memory, &ockam_responder_ip);
  // error = xx_test_initiator(&vault, &memory, &ockam_initiator_ip, &ockam_responder_ip);
  //  goto exit;

  responder_process = fork();
  if (responder_process < 0) {
    error = KEYAGREEMENT_ERROR_TEST;
    goto exit;
  }
  if (0 != responder_process) {
    if (run_initiator) {
      error = xx_test_initiator(&vault, &memory, &ockam_initiator_ip, &ockam_responder_ip);
      if (error) {
        initiator_status = -1;
        goto exit;
      }
    } // end if(run_initiator)
    // Get exit status from responder_process
    wait(&fork_status);
    responder_status = WEXITSTATUS(fork_status);
    if (responder_status) {
      responder_status = -2;
      goto exit;
    }
  } else {
    if (run_responder) {
      // This is the server process
      error = xx_test_responder(&vault, &memory, &ockam_responder_ip);
      if (error) goto exit;
    }
  }

exit:
  printf("Tests done\n");
  if (initiator_status) printf("Initiator failed.\n");
  if (responder_status) printf("Responder failed.\n");
  if (error) ockam_log_error("%x", error);
  return error;
}
