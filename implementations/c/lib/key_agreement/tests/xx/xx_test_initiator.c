
#include <stdio.h>
#include <string.h>

#include "ockam/error.h"
#include "ockam/key_agreement.h"
#include "../../xx/xx_local.h"
#include "ockam/memory.h"
#include "ockam/syslog.h"
#include "ockam/transport.h"
#include "ockam/vault.h"
#include "xx_test.h"

OckamError OckamErrorXXTestInitiatorPrologue(KeyEstablishmentXX *xx) {
  OckamError status = kOckamErrorNone;
  uint8_t key[KEY_SIZE];
  uint32_t key_bytes;

  // 1. Pick a static 25519 keypair for this handshake and set it to s
  string_to_hex(INITIATOR_STATIC, key, &key_bytes);
  status = xx->vault->KeySetPrivate(xx->vault_ctx, kOckamVaultKeyStatic, key, KEY_SIZE);
  if (kOckamErrorNone != status) {
    log_error(status, "failed to generate static keypair in initiator_step_1");
    goto exit_block;
  }

  status = xx->vault->KeyGetPublic(xx->vault_ctx, kOckamVaultKeyStatic, xx->s, KEY_SIZE);
  if (kOckamErrorNone != status) {
    log_error(status, "failed to generate get static public key in initiator_step_1");
    goto exit_block;
  }

  // 2. Generate an ephemeral 25519 keypair for this handshake and set it to e
  string_to_hex(INITIATOR_EPH, key, &key_bytes);
  status = xx->vault->KeySetPrivate(xx->vault_ctx, kOckamVaultKeyEphemeral, key, KEY_SIZE);
  if (kOckamErrorNone != status) {
    log_error(status, "failed to generate static keypair in initiator_step_1");
    goto exit_block;
  }

  status = xx->vault->KeyGetPublic(xx->vault_ctx, kOckamVaultKeyEphemeral, xx->e, KEY_SIZE);
  if (kOckamErrorNone != status) {
    log_error(status, "failed to generate get static public key in initiator_step_1");
    goto exit_block;
  }

  // Nonce to 0, k to empty
  xx->nonce = 0;
  memset(xx->k, 0, sizeof(xx->k));

  // Initialize h to "Noise_XX_25519_AESGCM_SHA256" and set prologue to empty
  memset(&xx->h[0], 0, SHA256_SIZE);
  memcpy(&xx->h[0], PROTOCOL_NAME, PROTOCOL_NAME_SIZE);

  // Initialize ck
  memset(&xx->ck[0], 0, SHA256_SIZE);
  memcpy(&xx->ck[0], PROTOCOL_NAME, PROTOCOL_NAME_SIZE);

  // h = SHA256(h || prologue), prologue is empty
  mix_hash(xx, NULL, 0);

exit_block:
  return status;
}

/**
 ********************************************************************************************************
 *                                          TestInitiatorHandshake ()
 ********************************************************************************************************
 *
 * Summary: Test the handshake process by starting with predefined static and
 *ephemeral keys (generated in the prologue) and verifying intermediate results
 *against test data along the way
 *
 * @param transportCtx [in] - initialized transport transportCtx
 * @param xx [in/out] - pointer to handshake structure
 * @return [out] - kErrorNone on success
 ********************************************************************************************************
 */

OckamError TestInitiatorHandshake(const OckamVault *vault, OckamVaultCtx *vaultCtx, const OckamTransport *transport,
                                  OckamTransportCtx transportCtx, KeyEstablishmentXX *xx) {
  OckamError status = kErrorNone;
  uint8_t sendBuffer[MAX_TRANSMIT_SIZE];
  uint8_t recv_buffer[MAX_TRANSMIT_SIZE];
  uint16_t bytesReceived = 0;
  uint16_t transmit_size = 0;
  uint8_t compare[1024];
  uint32_t compare_bytes;

  /* Initialize the KeyEstablishmentXX struct */
  memset(xx, 0, sizeof(*xx));
  OckamKeyInitializeXX(xx, vault, vaultCtx, transport, transportCtx);

  /* Prologue initializes keys and handshake parameters */
  status = OckamErrorXXTestInitiatorPrologue(xx);
  if (status != kErrorNone) {
    log_error(status, "TestInitiatorPrologue");
    goto exit_block;
  }

  // Step 1 generate message
  status = XXInitiatorM1Make(xx, sendBuffer, MAX_TRANSMIT_SIZE, &transmit_size);
  if (kErrorNone != status) {
    log_error(status, "initiator_step_1 failed");
    goto exit_block;
  }

  // Verify
  string_to_hex(MSG_1_CIPHERTEXT, compare, &compare_bytes);
  if (0 != memcmp(sendBuffer, compare, compare_bytes)) {
    status = kXXKeyAgreementTestFailed;
    log_error(status, "Test failed on msg 0\n");
    goto exit_block;
  }

  // Step 1 send message
  status = xx->transport->Write(transportCtx, sendBuffer, transmit_size);
  if (kErrorNone != status) {
    log_error(status, "ockam_SendBlocking after initiator_step_1 failed");
    goto exit_block;
  }

  // Msg 2 receive
  status = xx->transport->Read(transportCtx, recv_buffer, sizeof(recv_buffer), &bytesReceived);
  if (kErrorNone != status) {
    log_error(status, "ockam_ReceiveBlocking failed on msg 2");
    goto exit_block;
  }

  // Msg 2 process
  status = XXInitiatorM2Process(xx, recv_buffer, bytesReceived);
  if (kErrorNone != status) {
    log_error(status, "ockam_ReceiveBlocking failed on msg 2");
    goto exit_block;
  }

  // Msg 3 make
  status = XXInitiatorM3Make(xx, sendBuffer, &transmit_size);
  if (kErrorNone != status) {
    log_error(status, "initiator_m3_make failed");
    goto exit_block;
  }

  /* Msg 3 verify */
  string_to_hex(MSG_3_CIPHERTEXT, compare, &compare_bytes);
  if (0 != memcmp(compare, sendBuffer, transmit_size)) {
    status = kXXKeyAgreementTestFailed;
    log_error(status, "-------Msg 3 verify failed");
    goto exit_block;
  }

  // Msg 3 send
  status = xx->transport->Write(transportCtx, sendBuffer, transmit_size);
  if (kErrorNone != status) {
    log_error(status, "ockam_SendBlocking failed on msg 3");
    goto exit_block;
  }

  status = XXInitiatorEpilogue(xx);
  if (kErrorNone != status) {
    log_error(status, "initiator_epilogue failed");
    goto exit_block;
  }

exit_block:
  return status;
}

OckamError EstablishInitiatorConnection(int argc, char *argv[], const OckamTransport *transport,
                                        OckamTransportCtx *transportCtx) {
  OckamError status = kErrorNone;
  OckamInternetAddress responder_address;
  OckamTransportConfig tcpConfig = {kBlocking};

  // Get the IP address of the responder
  status = GetIpInfo(argc, argv, &responder_address);
  if (kErrorNone != status) {
    log_error(status, "failed to get address into");
    goto exit_block;
  }

  // Initialize TCP transportCtx
  status = transport->Create(transportCtx, &tcpConfig);
  if (kErrorNone != status) {
    log_error(status, "failed transport->create");
    goto exit_block;
  }

  // Try to connect
  status = transport->Connect(*transportCtx, &responder_address);
  if (kErrorNone != status) {
    log_error(status, "connect failed");
    goto exit_block;
  }

exit_block:
  return status;
}

extern const OckamTransport ockamPosixTcpTransport;

OckamError XXTestInitiator(int argc, char *argv[], const OckamVault *vault, void *vault_ctx) {
  const OckamTransport *transport = &ockamPosixTcpTransport;

  OckamError status = kErrorNone;
  OckamTransportCtx transportCtx;
  KeyEstablishmentXX handshake;
  uint8_t sendBuffer[MAX_TRANSMIT_SIZE];
  uint8_t recv_buffer[MAX_TRANSMIT_SIZE];
  uint16_t bytesReceived = 0;
  uint16_t transmit_size = 0;
  uint8_t test[TEST_MSG_CIPHER_SIZE];
  uint32_t test_bytes;
  uint8_t test_responder[TEST_MSG_CIPHER_SIZE];

  /*-------------------------------------------------------------------------
   * Establish transport transportCtx with responder
   *-----------------------------------------------------------------------*/
  status = EstablishInitiatorConnection(argc, argv, transport, &transportCtx);
  if (kErrorNone != status) {
    log_error(status, "Failed to establish transportCtx with responder");
    goto exit_block;
  }

  /*-------------------------------------------------------------------------
   * Secure the transportCtx
   *-----------------------------------------------------------------------*/
  status = TestInitiatorHandshake(vault, vault_ctx, transport, transportCtx, &handshake);
  if (kErrorNone != status) {
    log_error(status, "ockam_initiator_handshake");
    goto exit_block;
  }

  /*-------------------------------------------------------------------------
   * Receive the test message
   *-----------------------------------------------------------------------*/
  status = transport->Read(transportCtx, recv_buffer, sizeof(recv_buffer), &bytesReceived);
  if (kErrorNone != status) {
    log_error(status, "ockam_ReceiveBlocking failed on test message");
    goto exit_block;
  }

  /*-------------------------------------------------------------------------
   * Confirm the test message
   *-----------------------------------------------------------------------*/
  status = XXDecrypt(&handshake, test, TEST_MSG_BYTE_SIZE, recv_buffer, bytesReceived, &test_bytes);
  if (kErrorNone != status) {
    log_error(status, "XXDecrypt failed on test msg");
    goto exit_block;
  }
  string_to_hex(TEST_MSG_RESPONDER, test_responder, NULL);
  if (0 != memcmp((void *)test, test_responder, TEST_MSG_BYTE_SIZE)) {
    status = kXXKeyAgreementTestFailed;
    log_error(status, "Received bad epilogue message");
    goto exit_block;
  }

  /*-------------------------------------------------------------------------
   * Make the test message
   *-----------------------------------------------------------------------*/
  string_to_hex(TEST_MSG_INITIATOR, test, &test_bytes);
  status = XXEncrypt(&handshake, test, test_bytes, sendBuffer, sizeof(sendBuffer), &transmit_size);
  if (kErrorNone != status) {
    log_error(status, "initiator_encrypt failed on test message");
    goto exit_block;
  }

  /*-------------------------------------------------------------------------
   * Confirm the test message
   *-----------------------------------------------------------------------*/
  string_to_hex(MSG_5_CIPHERTEXT, test, &test_bytes);
  if (0 != memcmp(test, sendBuffer, transmit_size)) {
    status = kXXKeyAgreementTestFailed;
    log_error(status, "Msg 5 failed");
    goto exit_block;
  }

  /*-------------------------------------------------------------------------
   * Send the test message
   *-----------------------------------------------------------------------*/
  status = transport->Write(transportCtx, sendBuffer, transmit_size);
  if (kErrorNone != status) {
    log_error(status, "ockam_SendBlocking failed on test message");
    goto exit_block;
  }

exit_block:
  if (NULL != transportCtx) transport->Destroy(transportCtx);
  return status;
}
