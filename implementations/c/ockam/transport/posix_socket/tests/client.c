#include <unistd.h>
#include "ockam/error.h"
#include "ockam/log.h"
#include "ockam/io.h"
#include "ockam/transport.h"
//#include "ockam/transport/socket_tcp.h"
#include "ockam/transport/socket_udp.h"
#include "ockam/memory.h"
#include "tools.h"
#include <stdbool.h>

ockam_error_t run_test_client(test_cli_params_t* p_params)
{
  ockam_error_t error = ockam_transport_posix_socket_error_none;

  ockam_transport_t                   transport;
  ockam_transport_socket_attributes_t transport_attributes;
  ockam_reader_t*                     p_transport_reader;
  ockam_writer_t*                     p_transport_writer;

  error = ockam_memory_set(&p_params->memory, &transport_attributes, 0, sizeof(transport_attributes));
  if (ockam_error_has_error(&error)) goto exit;
  transport_attributes.p_memory = &p_params->memory;
  error                         = ockam_memory_copy(transport_attributes.p_memory,
                            &transport_attributes.local_address,
                            &p_params->client_address,
                            sizeof(p_params->client_address));
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_memory_copy(transport_attributes.p_memory,
                            &transport_attributes.remote_address,
                            &p_params->server_address,
                            sizeof(p_params->server_address));
  if (ockam_error_has_error(&error)) goto exit;

//  if (p_params->run_tcp_test) {
//    ockam_log_info("Running TCP Client Init");
//    error = ockam_transport_socket_tcp_init(&transport, &transport_attributes);
//  } else {
    ockam_log_info("Waiting UDP Server to start");
    sleep(2);
    ockam_log_info("Running UDP Client Init");
    error = ockam_transport_socket_udp_init(&transport, &transport_attributes);
//  }

  ockam_ip_address_t remote_address;
  error = ockam_memory_set(&p_params->memory, &remote_address, 0, sizeof(remote_address));
  if (ockam_error_has_error(&error)) goto exit;
  error = ockam_memory_copy(&p_params->memory,
                            remote_address.ip_address,
                            p_params->server_address.ip_address,
                            sizeof(p_params->server_address.ip_address));
  if (ockam_error_has_error(&error)) goto exit;
  remote_address.port = p_params->server_address.port;

  ockam_log_info("Running client connect");
  error = ockam_transport_connect(&transport, &p_transport_reader, &p_transport_writer, 10, 1);
  if (ockam_error_has_error(&error)) goto exit;
  ockam_log_info("Client connect finished");

  FILE* p_file_to_send;

  error = open_file_for_client_send(p_params->fixture_path, &p_file_to_send);
  if (ockam_error_has_error(&error)) goto exit;

  sleep(1);
  while (true) {
    if (feof(p_file_to_send)) { break; }

    uint8_t send_buffer[64];
    size_t  send_length = fread(send_buffer, 1, sizeof(send_buffer), p_file_to_send);

    error = ockam_write(p_transport_writer, send_buffer, send_length);
    ockam_log_debug("Client loop write finish");
    if (ockam_error_has_error(&error)) {
      ockam_log_error("%s", "Send failed");
      goto exit;
    }
    usleep(750);
  }

  fclose(p_file_to_send);

  // Send special "the end" buffer
  error = ockam_write(p_transport_writer, (uint8_t*) ENDING_LINE, strlen(ENDING_LINE) + 1);
  if (ockam_error_has_error(&error)) {
    ockam_log_error("%s", "Send failed");
    goto exit;
  }

  ockam_log_info("Client file send finished");

  FILE* p_file_to_receive;
  error = open_file_for_client_receive(p_params->fixture_path, &p_file_to_receive);
  if (ockam_error_has_error(&error)) goto exit;

  // Receive the test data file
  while (true) {
    size_t  bytes_received = 0;
    uint8_t receive_buffer[64];

    do {
      ockam_log_debug("Client loop read start");
      error = ockam_read(p_transport_reader, &receive_buffer[0], sizeof(receive_buffer), &bytes_received);
      if (error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
          && OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN == error.domain) {
        usleep(500);
      }
      else if (ockam_error_has_error(&error)) {
        ockam_log_error("%s", "Receive failed");
        goto exit;
      }
      ockam_log_debug("Client loop read finish");
    } while (ockam_error_has_error(&error));

    // Look for special "the end" buffer
    if (0 == strncmp(ENDING_LINE, (char*) receive_buffer, strlen(ENDING_LINE))) {
      break;
    } else {
      size_t bytes_written = fwrite(&receive_buffer[0], 1, bytes_received, p_file_to_receive);
      if (bytes_written != bytes_received) {
        ockam_log_error("%s", "failed write to output file");
        error.code = -1;
        goto exit;
      }
    }
  }

  fclose(p_file_to_receive);

  FILE* p_sent_file;
  FILE* p_received_file;

  error = open_files_for_client_compare(p_params->fixture_path, &p_sent_file, &p_received_file);
  if (ockam_error_has_error(&error)) goto exit;

  // Now compare the received file and the reference file
  error = file_compare(&p_params->memory, p_sent_file, p_received_file);
  if (ockam_error_has_error(&error)) {
    ockam_log_error("%s", "file compare failed");
    goto exit;
  }
  printf("Client test successful!\n");

  fclose(p_sent_file);
  fclose(p_received_file);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}
