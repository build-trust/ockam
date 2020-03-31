#include <stdio.h>
#include <string.h>
#include <unistd.h>

#include "ockam/syslog.h"
#include "ockam/transport.h"
#include "test_tcp.h"

char *pFileToSend = "fixtures/client_test_data.txt";
char *pFileToReceive = "fixtures/client_data_received.txt";
char *pFileToCompare = "fixtures/server_test_data.txt";

#define DEFAULT_IP_ADDRESS "127.0.0.1"
#define DEFAULT_IP_PORT 8000

TransportError GetIpInfo(int argc, char *argv[], OckamInternetAddress *p_address) {
  TransportError status = kErrorNone;

  memset(p_address, 0, sizeof(*p_address));

  if (3 != argc) {
    strcpy(p_address->IPAddress, DEFAULT_IP_ADDRESS);
    p_address->port = DEFAULT_IP_PORT;
  } else {
    strcpy(p_address->IPAddress, argv[1]);
    p_address->port = strtoul(argv[2], NULL, 0);
  }

exit_block:
  return status;
}

// This is the dispatch table (vtable) for posix TCP transports
extern OckamTransport ockamPosixTcpTransport;
const OckamTransport *transport = &ockamPosixTcpTransport;

int testTcpClient(OckamInternetAddress *pHostAddress) {
  TransportError status = kErrorNone;
  OckamTransportCtx connection = NULL;
  char sendBuffer[64];
  uint16_t sendLength;
  char receive_buffer[64];
  uint16_t bytesReceived = 0;
  FILE *fileToSend = NULL;
  FILE *fileToReceive = NULL;
  size_t bytesWritten;
  uint16_t sendNotDone = 1;
  uint16_t receiveNotDone = 1;
  OckamTransportConfig tcpConfig = {kBlocking};

  // Open the test data file for sending
  fileToSend = fopen(pFileToSend, "r");
  if (NULL == fileToSend) {
    status = kTestFailure;
    log_error(status, "failed to open test file test_data_client.txt");
    goto exit_block;
  }

  // Create file for test data received
  fileToReceive = fopen(pFileToReceive, "w");
  if (NULL == fileToReceive) {
    status = kTestFailure;
    log_error(status, "failed to open test file test_data_client.txt");
    goto exit_block;
  }

  // Initialize TCP connection
  status = transport->Create(&connection, &tcpConfig);
  if (kErrorNone != status) {
    log_error(status, "failed PosixTcpInitialize");
    goto exit_block;
  }

  // Try to connect
  status = transport->Connect(connection, pHostAddress);
  if (kErrorNone != status) {
    log_error(status, "connect failed");
    goto exit_block;
  }

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

  // Receive the test data file
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

  fclose(fileToSend);
  fclose(fileToReceive);

  // Now compare the received file and the reference file
  if (0 != file_compare(pFileToReceive, pFileToCompare)) {
    status = kTestFailure;
    log_error(status, "file compare failed");
    goto exit_block;
  }

exit_block:
  if (NULL != connection) transport->Destroy(connection);
  return status;
}

int main(int argc, char *argv[]) {
  TransportError status;
  int testServerStatus = 0;
  int testClientStatus = 0;
  int forkStatus = 0;
  int32_t testServerProcess = 0;
  OckamInternetAddress ipAddress;

  status = GetIpInfo(argc, argv, &ipAddress);
  if (kErrorNone != status) {
    log_error(status, "failed to get address info");
    goto exit_block;
  }
  testServerProcess = fork();
  if (testServerProcess < 0) {
    log_error(kTestFailure, "Fork unsuccessful");
    status = -1;
    goto exit_block;
  }
  if (0 != testServerProcess) {
    // This is the client process, give the server a moment to come to life
    sleep(3);
    status = testTcpClient(&ipAddress);
    if (0 != status) {
      log_error(kTestFailure, "testTcpClient failed");
      testClientStatus = -1;
    }
    // Get exit status from testServerProcess
    wait(&forkStatus);
    testServerStatus = WEXITSTATUS(forkStatus);
    if (0 != testServerStatus) {
      testServerStatus = -2;
    }
    status = testServerStatus + testClientStatus;
  } else {
    // This is the server process
    status = testTcpServer(&ipAddress);
    if (0 != status) {
      log_error(kTestFailure, "testTcpServer failed");
      status = -1;
    }
  }

exit_block:
  return status;
}
