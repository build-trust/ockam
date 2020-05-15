#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <getopt.h>
#include <sys/wait.h>
#include "ockam/error.h"
#include "ockam/syslog.h"
#include "ockam/io.h"
#include "ockam/transport.h"
#include "tests.h"

#define DEFAULT_FIXTURE_PATH  "fixtures"
#define DEFAULT_IP_ADDRESS    "127.0.0.1"
#define DEFAULT_IP_PORT       8000
#define FIXTURE_PATH_LEN      192
#define FIXTURE_FULL_PATH_LEN 256

char* p_file_to_send    = "client_test_data.txt";
char* p_file_to_receive = "server_data_received.txt";
char* p_file_to_compare = "server_test_data.txt";

ockam_transport_tcp_socket_attributes_t transport_attributes;

int test_tcp_client(ockam_ip_address_t* address, char* p_fixture_path)
{
  int                error = TRANSPORT_ERROR_TEST;
  ockam_transport_t* transport;
  ockam_reader_t*    p_transport_reader;
  ockam_writer_t*    p_transport_writer;
  uint8_t            send_buffer[64];
  size_t             send_length;
  uint8_t            receive_buffer[64];
  size_t             bytes_received  = 0;
  FILE*              file_to_send    = NULL;
  FILE*              file_to_receive = NULL;
  size_t             bytes_written;
  uint16_t           send_not_done                               = 1;
  uint16_t           receive_not_done                            = 1;
  char               file_to_send_path[FIXTURE_FULL_PATH_LEN]    = { 0 };
  char               file_to_receive_path[FIXTURE_FULL_PATH_LEN] = { 0 };
  char               file_to_compare_path[FIXTURE_FULL_PATH_LEN] = { 0 };

  // Open the test data file for sending
  sprintf(&file_to_send_path[0], "%s/%s", p_fixture_path, p_file_to_send);
  file_to_send = fopen(&file_to_send_path[0], "r");
  if (NULL == file_to_send) {
    log_error(error, "failed to open client test data");
    goto exit;
  }
  // Create file for test data received
  sprintf(&file_to_receive_path[0], "%s/%s", p_fixture_path, p_file_to_receive);
  file_to_receive = fopen(&file_to_receive_path[0], "w");
  if (NULL == file_to_send) {
    log_error(error, "failed to open client_data_received.txt");
    goto exit;
  }

  memset(&transport_attributes, 0, sizeof(transport_attributes));
  error = ockam_transport_socket_tcp_init(&transport, &transport_attributes);
  if (error) goto exit;
  error = ockam_transport_connect(transport, &p_transport_reader, &p_transport_writer, address);
  if (error) goto exit;

  while (send_not_done) {
    send_length = fread(&send_buffer[0], 1, sizeof(send_buffer), file_to_send);
    if (feof(file_to_send)) send_not_done = 0;
    error = ockam_write(p_transport_writer, send_buffer, send_length);
    if (error) {
      log_error(error, "Send failed");
      goto exit;
    }
  }

  // Send special "the end" buffer
  error = ockam_write(p_transport_writer, (uint8_t*) "that's all", strlen("that's all") + 1);
  if (error) {
    log_error(error, "Send failed");
    goto exit;
  }

  // Receive the test data file
  while (receive_not_done) {
    error = ockam_read(p_transport_reader, &receive_buffer[0], sizeof(receive_buffer), &bytes_received);
    if (TRANSPORT_ERROR_NONE != error) {
      log_error(error, "Receive failed");
      goto exit;
    }
    // Look for special "the end" buffer
    if (0 == strncmp("that's all", (char*) receive_buffer, strlen("that's all"))) {
      receive_not_done = 0;
    } else {
      bytes_written = fwrite(&receive_buffer[0], 1, bytes_received, file_to_receive);
      if (bytes_written != bytes_received) {
        log_error(TRANSPORT_ERROR_TEST, "failed write to output file");
        goto exit;
      }
    }
  }

  fclose(file_to_send);
  fclose(file_to_receive);

  // Now compare the received file and the reference file
  sprintf(file_to_compare_path, "%s/%s", p_fixture_path, p_file_to_compare);
  if (0 != file_compare(file_to_receive_path, file_to_compare_path)) {
    error = TRANSPORT_ERROR_TEST;
    log_error(error, "file compare failed");
    goto exit;
  }
  printf("Client test successful!\n");

exit:
  return error;
}

static struct option long_options[] = {
  { "ip", required_argument, 0, 'i' },
  { "port", required_argument, 0, 'p' },
  { "fixture_path", required_argument, 0, 'f' },
};

void process_opts(int argc, char* argv[], ockam_ip_address_t* p_address, char* p_fixture_path)
{
  int8_t ch;

  while ((ch = getopt_long(argc, argv, "i:p:f:", long_options, NULL)) != -1) {
    switch (ch) {
    case 'i':
      strcpy((char*) p_address->ip_address, optarg);
      break;
    case 'p':
      p_address->port = strtoul(optarg, NULL, 0);
      break;
    case 'f':
      strncpy(p_fixture_path, optarg, FIXTURE_PATH_LEN);
      break;
    default:
      break;
    }
  }
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

  ip_address.port = DEFAULT_IP_PORT;
  strcpy((char*) &(ip_address.ip_address)[0], DEFAULT_IP_ADDRESS);
  strcpy(fixture_path, DEFAULT_FIXTURE_PATH);

  process_opts(argc, argv, &ip_address, fixture_path);

  test_server_process = fork();
  if (test_server_process < 0) {
    log_error(TRANSPORT_ERROR_TEST, "Fork unsuccessful");
    error = -1;
    goto exit;
  }
  if (0 != test_server_process) {
    // This is the client process, give the server a moment to come to life
    sleep(1);
    error = test_tcp_client(&ip_address, &fixture_path[0]);
    if (0 != error) {
      log_error(TRANSPORT_ERROR_TEST, "testTcpClient failed");
      test_client_error = -1;
    }
    // Get exit error from testServerProcess
    wait(&fork_error);
    test_server_error = WEXITSTATUS(fork_error);
    if (0 != test_server_error) { test_server_error = -2; }
    error = test_server_error + test_client_error;
    if (!error) printf("Transport test successful!\n");
  } else {
    // This is the server process
    error = test_tcp_server(&ip_address, &fixture_path[0]);
    if (0 != error) {
      log_error(TRANSPORT_ERROR_TEST, "testTcpServer failed");
      error = -1;
    }
  }

exit:
  return error;
}