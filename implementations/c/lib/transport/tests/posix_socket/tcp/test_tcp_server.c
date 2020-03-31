#include <stdio.h>

#include "ockam/error.h"
#include "ockam/syslog.h"
#include "ockam/transport.h"
#include "test_tcp.h"

char *pSrvFileToSend = "fixtures/server_test_data.txt";
char *pSrvFileToReceive = "fixtures/server_data_received.txt";
char *pSrvFileToCompare = "fixtures/client_test_data.txt";

extern const OckamTransport *transport;

int testTcpServer(OckamInternetAddress *pIPAddress) {
  TransportError status = kErrorNone;
  OckamTransportCtx connection = NULL;
  OckamTransportCtx listener = NULL;
  char sendBuffer[64];
  unsigned sendLength;
  char receive_buffer[64];
  uint16_t bytesReceived = 0;
  FILE *fileToSend = NULL;
  FILE *fileToReceive = NULL;
  FILE *errorLog = NULL;
  uint16_t bytesWritten;
  unsigned sendNotDone = 1;
  unsigned receiveNotDone = 1;
  OckamTransportConfig tcpConfig = {kBlocking};

  // Initialize TCP connection
  status = transport->Create(&listener, &tcpConfig);
  if (kErrorNone != status) {
    log_error(status, "failed PosixTcpInitialize");
    goto exit_block;
  }

  // Open the test data file for sending
  fileToSend = fopen(pSrvFileToSend, "r");
  if (NULL == fileToSend) {
    status = kTestFailure;
    log_error(status, "failed to open test file test_data_client.txt");
    goto exit_block;
  }

  // Create file for test data received
  fileToReceive = fopen(pSrvFileToReceive, "w");
  if (NULL == fileToReceive) {
    status = kTestFailure;
    log_error(status, "failed to open test file test_data_client.txt");
    goto exit_block;
  }

  // Listen (blocking) for a connection
  status = transport->Listen(listener, pIPAddress, &connection);
  if (kErrorNone != status) {
    log_error(status, "listen failed");
    goto exit_block;
  }
  while (receiveNotDone) {
    status = transport->Read(connection, &receive_buffer[0], sizeof(receive_buffer), &bytesReceived);
    if (kErrorNone != status) {
      log_error(status, "Receive failed");
      goto exit_block;
    }
    // Look for special "the end" buffer
    if (0 == strncmp("that's all", &receive_buffer[0], strlen("that's all"))) {
      receiveNotDone = 0;
    } else {
      bytesWritten = fwrite(&receive_buffer[0], 1, bytesReceived, fileToReceive);
      if (bytesWritten != bytesReceived) {
        log_error(kTestFailure, "failed write to output file");
        goto exit_block;
      }
    }
  }

  fclose(fileToReceive);

  // Send the test data file
  while (sendNotDone) {
    sendLength = fread(&sendBuffer[0], 1, sizeof(sendBuffer), fileToSend);
    if (feof(fileToSend)) sendNotDone = 0;
    status = transport->Write(connection, &sendBuffer[0], sendLength);
    if (kErrorNone != status) {
      log_error(status, "Send failed");
      goto exit_block;
    }
  }
  // Send special "the end" buffer
  status = transport->Write(connection, "that's all", strlen("that's all") + 1);
  if (kErrorNone != status) {
    log_error(status, "Send failed");
    goto exit_block;
  }

  fclose(fileToSend);

  // Now compare the received file and the reference file
  if (0 != file_compare(pSrvFileToReceive, pSrvFileToCompare)) {
    status = kTestFailure;
    log_error(status, "file compare failed");
    goto exit_block;
  }

exit_block:
  if (NULL != connection) transport->Destroy(connection);
  if (NULL != listener) transport->Destroy(listener);

  fclose(errorLog);
  printf("Exiting with status %.8x\n", status);
  return status;
}
