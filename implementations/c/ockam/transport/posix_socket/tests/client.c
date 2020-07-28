#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include "ockam/error.h"
#include "ockam/log/syslog.h"
#include "ockam/io.h"
#include "ockam/transport.h"
#include "ockam/transport/socket_tcp.h"
#include "ockam/transport/socket_udp.h"
#include "ockam/memory.h"
#include "tools.h"
#include <stdbool.h>

int run_test_client(test_cli_params_t* p_params)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  ockam_transport_t                   transport;
  ockam_transport_socket_attributes_t transport_attributes;
  ockam_reader_t*                     p_transport_reader;
  ockam_writer_t*                     p_transport_writer;

  error = ockam_memory_set(&p_params->memory, &transport_attributes, 0, sizeof(transport_attributes));
  if (error) goto exit;
  transport_attributes.p_memory = &p_params->memory;
  error = ockam_memory_copy(transport_attributes.p_memory, &transport_attributes.listen_address, &p_params->client_address, sizeof(p_params->client_address));
  if (error) goto exit;

  if (p_params->run_tcp_test) {
    printf("Running TCP Client Test\n");
    error = ockam_transport_socket_tcp_init(&transport, &transport_attributes);
  } else {
    printf("Running UDP Client Test\n");
    sleep(2);
    error = ockam_transport_socket_udp_init(&transport, &transport_attributes);
  }
  if (error) goto exit;

  ockam_ip_address_t remote_address;
  error = ockam_memory_set(&p_params->memory, &remote_address, 0, sizeof(remote_address));
  if (error) goto exit;
  error = ockam_memory_copy(&p_params->memory,
                            remote_address.ip_address,
                            p_params->server_address.ip_address,
                            sizeof(p_params->server_address.ip_address));
  if (error) goto exit;
  remote_address.port = p_params->server_address.port;

  error = ockam_transport_connect(&transport, &p_transport_reader, &p_transport_writer, &remote_address, 10, 1);
  if (error) goto exit;

  FILE* p_file_to_send;

  error = open_file_for_client_send(p_params->fixture_path, &p_file_to_send);
  if (error) goto exit;

  while (true) {
    if (feof(p_file_to_send)) {
      break;
    }

    uint8_t send_buffer[64];
    size_t send_length = fread(send_buffer, 1, sizeof(send_buffer), p_file_to_send);
    error = ockam_write(p_transport_writer, send_buffer, send_length);
    if (error) {
      log_error(error, "Send failed");
      goto exit;
    }
  }

  fclose(p_file_to_send);

  // Send special "the end" buffer
  error = ockam_write(p_transport_writer, (uint8_t*) "that's all", strlen("that's all") + 1);
  if (error) {
    log_error(error, "Send failed");
    goto exit;
  }

  FILE* p_file_to_receive;
  error = open_file_for_client_receive(p_params->fixture_path, &p_file_to_receive);
  if (error) goto exit;

  // Receive the test data file
  while (true) {
    size_t      bytes_received = 0;
    uint8_t receive_buffer[64];

    error = ockam_read(p_transport_reader, &receive_buffer[0], sizeof(receive_buffer), &bytes_received);
    if (TRANSPORT_ERROR_NONE != error) {
      log_error(error, "Receive failed");
      goto exit;
    }
    // Look for special "the end" buffer
    if (0 == strncmp(ENDING_LINE, (char*) receive_buffer, strlen(ENDING_LINE))) {
      break;
    } else {
      size_t bytes_written = fwrite(&receive_buffer[0], 1, bytes_received, p_file_to_receive);
      if (bytes_written != bytes_received) {
        log_error(TRANSPORT_ERROR_TEST, "failed write to output file");
        goto exit;
      }
    }
  }

  fclose(p_file_to_receive);

  FILE* p_sent_file;
  FILE* p_received_file;

  error = open_files_for_client_compare(p_params->fixture_path, &p_sent_file, &p_received_file);
  if (error) goto exit;

  // Now compare the received file and the reference file
  if (0 != file_compare(&p_params->memory, p_sent_file, p_received_file)) {
    error = TRANSPORT_ERROR_TEST;
    log_error(error, "file compare failed");
    goto exit;
  }
  printf("Client test successful!\n");

  fclose(p_sent_file);
  fclose(p_received_file);

exit:
  return error;
}
