
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <unistd.h>

#include "ockam/error.h"
#include "ockam/key_agreement.h"
#include "../../xx/xx_local.h"
#include "ockam/memory.h"
#include "ockam/syslog.h"
#include "ockam/transport.h"
#include "ockam/vault.h"
#include "xx_test.h"
//!!
#include "../../../../vault/default/default.h"

#define ACK "ACK"
#define ACK_SIZE 3
#define OK "OK"
#define OK_SIZE 2

bool scripted_xx = false;
bool run_initiator = false;
bool run_responder = false;
OckamInternetAddress ockam_ip = {"", "127.0.0.1", 8000};

void usage() {
  printf("OPTIONS\n");
  printf("  -a<xxx.xxx.xxx.xxx>\t\tIP Address\n");
  printf("  -p<portnum>\t\t\tPort\n");
  printf("  -i \t\t\t\tRun initiator only\n");
  printf("  -r \t\t\t\tRun responder only \n");
  printf("  -s \t\t\t\tUse scripted test case\n\n");
}

OckamError parse_opts(int argc, char *argv[]) {
  int ch;
  OckamError status = kOckamErrorNone;
  while ((ch = getopt(argc, argv, "hsira:p:")) != -1) {
    switch (ch) {
      case 'h':
        usage();
        return 2;

      case 'a':
        strcpy(ockam_ip.IPAddress, optarg);
        break;

      case 'p':
        ockam_ip.port = atoi(optarg);
        break;

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
        status = kBadParameter;
        usage();
        log_error(status, "invalid command-line arguments");
        return 2;

      default:
        break;
    }
  }

  return status;
}

const OckamMemory *memory = &ockam_memory_stdlib;
extern OckamTransport ockamPosixTcpTransport;
OckamVaultDefaultConfig default_cfg = {.features = OCKAM_VAULT_ALL, .ec = kOckamVaultEcCurve25519};

int main(int argc, char *argv[]) {
  const OckamVault *vault = &ockam_vault_default;
  const OckamTransport *transport = &ockamPosixTcpTransport;

  int responder_status = 0;
  int initiator_status = 0;
  int fork_status = 0;
  int32_t responder_process = 0;

  OckamError status = kErrorNone;
  void *vault_ctx = NULL;

  /*-------------------------------------------------------------------------
   * Parse options
   *-----------------------------------------------------------------------*/
  status = parse_opts(argc, argv);
  if (kOckamErrorNone != status) {
    log_error(status, "Invalid command line args");
    goto exit_block;
  }
  printf("Address: %s\n", ockam_ip.IPAddress);
  printf("Port: %u\n", ockam_ip.port);
  printf("Initiator: %d\n", run_initiator);
  printf("Responder: %d\n", run_responder);

  /*-------------------------------------------------------------------------
   * Initialize the vault
   *-----------------------------------------------------------------------*/
  memory->Create(0);
  status = vault->Create(&vault_ctx, &default_cfg, memory);
  if (status != kErrorNone) {
    log_error(status, "ockam_vault_init failed");
    goto exit_block;
  }

  responder_process = fork();
  if (responder_process < 0) {
    log_error(kTestFailure, "Fork unsuccessful");
    status = -1;
    goto exit_block;
  }
  if (0 != responder_process) {
    // This is the initiator process, give the server a moment to come to life
    if (run_initiator) {
      sleep(1);
      status = XXTestInitiator(vault, vault_ctx);
      if (0 != status) {
        log_error(kTestFailure, "testTcpClient failed");
        initiator_status = -1;
      }
    }  // end if(run_initiator)
    // Get exit status from responder_process
    wait(&fork_status);
    responder_status = WEXITSTATUS(fork_status);
    if (0 != responder_status) {
      responder_status = -2;
    }
    status = responder_status + initiator_status;
  } else {
    if (run_responder) {
      // This is the server process
      status = XXTestResponder(vault, vault_ctx);
      if (0 != status) {
        log_error(kTestFailure, "testTcpServer failed");
        status = -1;
      }
    }
  }

exit_block:
  printf("Test ended with status %0.4x\n", status);
  return status;
}
