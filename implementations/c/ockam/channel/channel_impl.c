#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <ctype.h>
#include <unistd.h>
#include "ockam/log.h"
#include "ockam/memory.h"
#include "ockam/key_agreement.h"
#include "ockam/key_agreement/xx.h"
#include "ockam/transport.h"
#include "ockam/io/impl.h"
#include "ockam/channel.h"
#include "ockam/channel/channel_impl.h"
#include "ockam/codec.h"

extern void print_uint8_str(uint8_t* p, uint16_t size, char* msg);

const char* const OCKAM_CHANNEL_INTERFACE_ERROR_DOMAIN = "OCKAM_CHANNEL_INTERFACE_ERROR_DOMAIN";

const ockam_error_t ockam_channel_interface_error_none = {
  OCKAM_ERROR_NONE,
  OCKAM_CHANNEL_INTERFACE_ERROR_DOMAIN
};

ockam_memory_t* gp_ockam_channel_memory = NULL;

ockam_error_t channel_read(void*, uint8_t*, size_t, size_t*);
ockam_error_t channel_write(void*, uint8_t*, size_t);
ockam_error_t channel_process_read(ockam_channel_t* ch, struct ockam_channel_poll_result* result);
ockam_error_t channel_process_system_message(ockam_channel_t* p_ch, uint8_t* p_encoded);
ockam_error_t
         channel_write_message(void* ctx, uint8_t* p_clear_text, size_t clear_text_length, codec_message_type_t message_type);
void     dump_route(codec_route_t* route);
uint8_t* route_encode(ockam_channel_t* ch, uint8_t* encoded);

uint8_t g_encoded_text[MAX_CHANNEL_PACKET_SIZE];

ockam_error_t channel_encrypt(ockam_channel_t* ch,
                              uint8_t*         clear_text,
                              size_t           clear_text_length,
                              uint8_t*         cipher_text,
                              size_t           cipher_text_size,
                              size_t*          cipher_text_length)
{
  ockam_error_t error = ockam_channel_interface_error_none;

  if (clear_text_length == 0) goto exit;

  if (ch->state == CHANNEL_STATE_SECURE) {
    error =
      ockam_key_encrypt(&ch->key, clear_text, clear_text_length, cipher_text, cipher_text_size, cipher_text_length);
  } else {
    error = ockam_memory_copy(gp_ockam_channel_memory, cipher_text, clear_text, clear_text_length);

    *cipher_text_length = clear_text_length;
  }
exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t channel_process_message(ockam_channel_t* ch, uint8_t* p_encoded)
{
  ockam_error_t        error        = ockam_channel_interface_error_none;
  codec_message_type_t message_type = *p_encoded++;
  switch (message_type) {
  case PAYLOAD: {
    uint8_t clear_text[MAX_CHANNEL_PACKET_SIZE];
    size_t  clear_text_length = 0;
    size_t  payload_length    = ch->channel_read_buffer_length - (p_encoded - ch->channel_read_buffer);
    error = ockam_key_decrypt(&ch->key, clear_text, sizeof(clear_text), p_encoded, payload_length, &clear_text_length);
    if (ockam_error_has_error(&error)) goto exit;
    p_encoded = clear_text;
    // Step over header
    // Onward addresses should be 0
    // Return address should be local only
    if (OCKAM_WIRE_PROTOCOL_VERSION != *p_encoded++) {
      error.code = -1;
      goto exit;
    }
    if (*p_encoded++ != 0) { // count of onward addresses
      error.code = -1;
      goto exit;
    }
    switch (*p_encoded++) {
    case 0:
      break;
    case 1:
      p_encoded += *p_encoded + 1; // step over local address & address type
      break;
    default:
      error.code = -1;
      goto exit;
    }
    // Step over message type
    ++p_encoded;
    ch->app_read_buffer_length = clear_text_length - (p_encoded - clear_text);
    ockam_memory_copy(gp_ockam_channel_memory, ch->app_read_buffer, p_encoded, ch->app_read_buffer_length);
    break;
  }
  default:
    error.code = OCKAM_CHANNEL_INTERFACE_ERROR_NOT_IMPLEMENTED;
    goto exit;
  }
exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t ockam_channel_init(ockam_channel_t* p_ch, ockam_channel_attributes_t* p_attrs)
{
  ockam_error_t error = ockam_channel_interface_error_none;
  printf("in %s\n", __func__);

  if ((NULL == p_ch) || (NULL == p_attrs) || (NULL == p_attrs->reader) || (NULL == p_attrs->writer) ||
      (NULL == p_attrs->memory) || (NULL == p_attrs->vault)) {
    error.code = -1;
    goto exit;
  }

  gp_ockam_channel_memory = p_attrs->memory;
  p_ch->vault             = p_attrs->vault;

  if ((p_attrs->local_host_address.type != ADDRESS_UDP) && (p_attrs->local_host_address.type != ADDRESS_TCP)) {
    error.code = -1;
    goto exit;
  }
  ockam_memory_copy(
    gp_ockam_channel_memory, &p_ch->local_host_address, &p_attrs->local_host_address, sizeof(p_ch->local_host_address));

  error = ockam_memory_alloc_zeroed(gp_ockam_channel_memory, (void**) &p_ch->channel_reader, sizeof(ockam_reader_t));
  if (ockam_error_has_error(&error)) goto exit;
  p_ch->channel_reader->read = channel_read;
  p_ch->channel_reader->ctx  = p_ch;

  error = ockam_memory_alloc_zeroed(gp_ockam_channel_memory, (void**) &p_ch->channel_writer, sizeof(ockam_writer_t));
  if (ockam_error_has_error(&error)) goto exit;
  p_ch->channel_writer->write = channel_write;
  p_ch->channel_writer->ctx   = p_ch;

  error =
    ockam_memory_alloc_zeroed(gp_ockam_channel_memory, (void**) &p_ch->channel_read_buffer, MAX_CHANNEL_PACKET_SIZE);
  if (ockam_error_has_error(&error)) goto exit;

  error =
    ockam_memory_alloc_zeroed(gp_ockam_channel_memory, (void**) &p_ch->channel_write_buffer, MAX_CHANNEL_PACKET_SIZE);
  if (ockam_error_has_error(&error)) goto exit;

  p_ch->transport_reader = p_attrs->reader;
  p_ch->transport_writer = p_attrs->writer;

  p_ch->onward_route.p_addresses = p_ch->onward_addresses;

  error = ockam_xx_key_initialize(&p_ch->key, gp_ockam_channel_memory, p_ch->vault);

exit:
  if (ockam_error_has_error(&error)) {
    ockam_log_error("%s: %d", error.domain, error.code);
    if (p_ch) {
      if (p_ch->channel_reader)
        ockam_memory_free(gp_ockam_channel_memory, (void*) p_ch->channel_reader, sizeof(ockam_reader_t));
      if (p_ch->channel_writer)
        ockam_memory_free(gp_ockam_channel_memory, (void*) p_ch->channel_writer, sizeof(ockam_writer_t));
      if (p_ch->channel_read_buffer) {
        ockam_memory_free(gp_ockam_channel_memory, p_ch->channel_read_buffer, MAX_CHANNEL_PACKET_SIZE);
      }
    }
  }
  return error;
}

ockam_error_t channel_initiate_key_exchange(ockam_channel_t* p_ch)
{
  ockam_error_t               error = ockam_channel_interface_error_none;
  uint8_t                     message[MAX_CHANNEL_PACKET_SIZE];
  size_t                      message_length = 0;
  ockam_channel_poll_result_t result         = { 0 };
  size_t                      offset         = 0;

  error = ockam_xx_key_initialize(&p_ch->key, gp_ockam_channel_memory, p_ch->vault);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_key_m1_make(&p_ch->key, message, sizeof(message), &message_length);
  if (ockam_error_has_error(&error)) goto exit;

  offset += message_length;

  printf("sending REQUEST_CHANNEL %d\n", REQUEST_CHANNEL);
  error = channel_write_message(p_ch, message, offset, REQUEST_CHANNEL);
  if (ockam_error_has_error(&error)) goto exit;

  do {
    usleep(500 * 1000);
    ockam_channel_poll(p_ch, &result);
  } while (CHANNEL_STATE_SECURE != p_ch->state);

  printf("Initiator secure\n");

exit:
  return error;
}

ockam_error_t
ockam_channel_connect(ockam_channel_t* ch, codec_route_t* route, ockam_reader_t** p_reader, ockam_writer_t** p_writer)
{
  ockam_error_t               error = ockam_channel_interface_error_none;
  ockam_channel_poll_result_t result;

  printf("in %s\n", __func__);

  ch->onward_route.count_addresses = route->count_addresses;
  ockam_memory_copy(gp_ockam_channel_memory,
                    ch->onward_route.p_addresses,
                    route->p_addresses,
                    ch->onward_route.count_addresses * sizeof(codec_address_t));

  error = channel_initiate_key_exchange(ch);
  if (ockam_error_has_error(&error)) goto exit;

  *p_reader = ch->channel_reader;
  *p_writer = ch->channel_writer;

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t channel_responder_key_exchange(ockam_channel_t* p_ch)
{
  ockam_error_t               error = ockam_channel_interface_error_none;
  uint8_t                     message[MAX_CHANNEL_PACKET_SIZE];
  size_t                      message_length = 0;
  ockam_channel_poll_result_t result;

  printf("in %s\n", __func__);
  do {
    error = ockam_channel_poll(p_ch, &result);
    if (ockam_error_has_error(&error)) {
      if (!(error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
            && error.domain == OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN)) goto exit;
      usleep(500 * 1000);
    }
  } while (p_ch->state != CHANNEL_STATE_SECURE);
  printf("responder secure\n");

exit:
  return error;
}

ockam_error_t ockam_channel_accept(ockam_channel_t* p_ch, ockam_reader_t** p_reader, ockam_writer_t** p_writer)
{
  ockam_error_t error = ockam_channel_interface_error_none;

  //  error = channel_responder_key_exchange(p_ch);
  //  if (error) goto exit;
  *p_reader = p_ch->channel_reader;
  *p_writer = p_ch->channel_writer;

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  error.domain = OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN;
  error.code = OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA;

  return error;
}

ockam_error_t channel_read(void* ctx, uint8_t* p_buffer, size_t buffer_size, size_t* p_buffer_length)
{
  ockam_error_t               error = ockam_channel_interface_error_none;
  ockam_channel_t*            p_ch  = (ockam_channel_t*) ctx;
  ockam_channel_poll_result_t result;

  if ((p_ch->app_read_buffer) && (p_ch->app_read_buffer != p_buffer)) {
    error.code = OCKAM_CHANNEL_INTERFACE_ERROR_READ_PENDING;
    goto exit;
  }

  p_ch->app_read_buffer      = p_buffer;
  p_ch->app_read_buffer_size = buffer_size;

  error = ockam_read(
    p_ch->transport_reader, p_ch->channel_read_buffer, MAX_CHANNEL_PACKET_SIZE, &p_ch->channel_read_buffer_length);
  if (ockam_error_has_error(&error)) goto exit;

  print_uint8_str(p_ch->channel_read_buffer, p_ch->channel_read_buffer_length, "channel_read");
  channel_process_read(p_ch, &result);
  *p_buffer_length             = result.bytes_read;
  p_ch->app_read_buffer_length = 0;
  p_ch->app_read_buffer        = 0;
  p_ch->app_read_buffer_size   = 0;

exit:
  if (!(error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
        && error.domain == OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t channel_process_read(ockam_channel_t* ch, struct ockam_channel_poll_result* result)
{
  ockam_error_t   error               = ockam_channel_interface_error_none;
  size_t          cipher_text_length  = 0;
  size_t          encoded_text_length = 0;
  uint8_t*        p_encoded           = ch->channel_read_buffer;
  codec_route_t   onward_route        = { 0 };
  codec_address_t onward_addresses[MAX_HOPS];

  p_encoded = decode_ockam_wire(p_encoded);

  ockam_memory_set(gp_ockam_channel_memory, (uint8_t*) onward_addresses, 0, sizeof(onward_addresses));
  onward_route.p_addresses = onward_addresses;

  p_encoded = decode_route(p_encoded, &onward_route);

  // decode return route into onward route for reply
  p_encoded = decode_route(p_encoded, &ch->onward_route);

  if (*p_encoded != PAYLOAD) {
    error               = channel_process_system_message(ch, p_encoded);
    result->read_buffer = 0;
    result->bytes_read  = 0;
  } else if (ch->onward_route.count_addresses <= 1) {
    uint16_t payload_length = 0;
    error                   = channel_process_message(ch, p_encoded);
    result->read_buffer     = ch->app_read_buffer;
    result->bytes_read      = ch->app_read_buffer_length;
  } else {
    error.code = OCKAM_CHANNEL_INTERFACE_ERROR_NOT_IMPLEMENTED;
    goto exit;
  }

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

int channel_process_ping()
{
  return 0;
}

ockam_error_t channel_m1_process(ockam_channel_t* p_ch, uint8_t* p_encoded)
{
  ockam_error_t error = ockam_channel_interface_error_none;
  uint8_t       m2_message[MAX_CHANNEL_PACKET_SIZE];
  size_t        m2_length = 0;

  error = ockam_key_m1_process(&p_ch->key, p_encoded);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_key_m2_make(&p_ch->key, m2_message, sizeof(m2_message), &m2_length);
  if (ockam_error_has_error(&error)) goto exit;

  printf("sending KEY_AGREEMENT_T1_M2 %d\n", KEY_AGREEMENT_T1_M2);
  error = channel_write_message(p_ch, m2_message, m2_length, KEY_AGREEMENT_T1_M2);
  if (ockam_error_has_error(&error)) goto exit;

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}
ockam_error_t channel_m2_process(ockam_channel_t* p_ch, uint8_t* p_encoded)
{
  ockam_error_t error = ockam_channel_interface_error_none;
  uint8_t       m3_message[MAX_CHANNEL_PACKET_SIZE];
  size_t        m3_length = 0;

  error = ockam_key_m2_process(&p_ch->key, p_encoded);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_key_m3_make(&p_ch->key, m3_message, sizeof(m3_message), &m3_length);
  if (ockam_error_has_error(&error)) goto exit;

  printf("sending KEY_AGREEMENT_T1_M3 %d\n", KEY_AGREEMENT_T1_M3);
  error = channel_write_message(p_ch, m3_message, m3_length, KEY_AGREEMENT_T1_M3);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_initiator_epilogue(&p_ch->key);
  if (ockam_error_has_error(&error)) goto exit;

  p_ch->state = CHANNEL_STATE_SECURE;

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}
ockam_error_t channel_m3_process(ockam_channel_t* p_ch, uint8_t* p_encoded)
{
  ockam_error_t error = ockam_channel_interface_error_none;
  uint8_t       m2_message[MAX_CHANNEL_PACKET_SIZE];
  size_t        m2_length = 0;

  printf("processing KEY_AGREEMENT_T1_M3 in %s\n", __func__);
  error = ockam_key_m3_process(&p_ch->key, p_encoded);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_responder_epilogue(&p_ch->key);
  if (ockam_error_has_error(&error)) goto exit;

  printf("responder secure\n");
  p_ch->state = CHANNEL_STATE_SECURE;

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t channel_process_system_message(ockam_channel_t* p_ch, uint8_t* p_encoded)
{
  ockam_error_t error = ockam_channel_interface_error_none;

  codec_message_type_t message_type = *p_encoded++;

  switch (message_type) {
  case PING:
    channel_process_ping();
    break;
  case REQUEST_CHANNEL:
    error = channel_m1_process(p_ch, p_encoded);
    break;
  case KEY_AGREEMENT_T1_M2:
    error = channel_m2_process(p_ch, p_encoded);
    break;
  case KEY_AGREEMENT_T1_M3:
    error = channel_m3_process(p_ch, p_encoded);
    break;
  case PONG:
    error.code = OCKAM_CHANNEL_INTERFACE_ERROR_NOT_IMPLEMENTED;
  default:
    break;
  }
  return error;
}

/* Write related functions start here */

uint8_t* route_encode(ockam_channel_t* ch, uint8_t* encoded)
{
  printf("Onward route:\n");
  dump_route(&ch->onward_route);
  encoded = encode_route(encoded, &ch->onward_route);

  /* Return route */
  codec_route_t route;
  route.p_addresses     = &ch->local_host_address;
  route.count_addresses = 1;
  encoded               = encode_route(encoded, &route);
  return encoded;
}

ockam_error_t channel_write_to_app(void* ctx, uint8_t* clear_text, size_t clear_text_length, int32_t remote_app_address)
{
  ockam_error_t   error   = ockam_channel_interface_error_none;
  codec_payload_t payload = { 0 };
  codec_route_t   route   = { 0 };
  codec_address_t addresses[2];
  uint8_t         message[MAX_CHANNEL_PACKET_SIZE];
  uint8_t*        encoded = message;

  //  route.count_addresses = 1;
  //  route.p_addresses = addresses;
  //  route.p_addresses[0].type = ADDRESS_LOCAL;
  //  route.p_addresses[0].address.local_address.size = sizeof(int32_t);
  //  *(int32_t*)route.p_addresses[0].address.local_address.address = remote_app_address;

  *encoded++ = 1; // ockam protocol v1
                  //  encoded = encode_route(encoded, &route);
  *encoded++ = 0; // no onward route
  *encoded++ = 0; // no return route
  *encoded++ = PAYLOAD;

  payload.data_length = clear_text_length;
  payload.data        = clear_text;
  encoded             = encode_payload(encoded, &payload);

  error = channel_write(ctx, message, encoded - message);

  return error;
}

ockam_error_t channel_write(void* ctx, uint8_t* p_clear_text, size_t clear_text_length)
{
  printf("In %s\n", __func__);
  return channel_write_message(ctx, p_clear_text, clear_text_length, PAYLOAD);
}

ockam_error_t
channel_write_message(void* ctx, uint8_t* p_clear_text, size_t clear_text_length, codec_message_type_t message_type)
{
  ockam_error_t    error              = ockam_channel_interface_error_none;
  size_t           cipher_text_length = 0;
  uint8_t*         p_encoded          = g_encoded_text;
  ockam_channel_t* p_ch               = (ockam_channel_t*) ctx;

  p_encoded    = encode_ockam_wire(p_encoded);
  p_encoded    = route_encode(p_ch, p_encoded);
  *p_encoded++ = message_type;

  error = channel_encrypt(p_ch,
                          p_clear_text,
                          clear_text_length,
                          p_encoded,
                          MAX_CHANNEL_PACKET_SIZE - (p_encoded - g_encoded_text),
                          &cipher_text_length);
  if (ockam_error_has_error(&error)) goto exit;
  p_encoded += cipher_text_length;

  error = ockam_write(p_ch->transport_writer, g_encoded_text, p_encoded - g_encoded_text);
  if (ockam_error_has_error(&error)) goto exit;

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t ockam_channel_poll(ockam_channel_t* p_ch, ockam_channel_poll_result_t* result)
{
  ockam_error_t error = ockam_channel_interface_error_none;
  ockam_memory_set(gp_ockam_channel_memory, result, 0, sizeof(*result));

  p_ch->channel_read_buffer_length = 0;
  error                            = ockam_read(
    p_ch->transport_reader, p_ch->channel_read_buffer, MAX_CHANNEL_PACKET_SIZE, &p_ch->channel_read_buffer_length);
  if (ockam_error_has_error(&error) &&
      !(OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA == error.code
        && OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN == error.domain)) goto exit;
  if (p_ch->channel_read_buffer_length > 0) {
    error = channel_process_read(p_ch, result);
    if (ockam_error_has_error(&error)) goto exit;
  }

  if (p_ch->app_write_buffer) {
    // !!TODO - asynchronous write polling
  }

  if (p_ch->state == CHANNEL_STATE_SECURE) result->channel_is_secure = 1;

exit:
  if (ockam_error_has_error(&error) && !(OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA == error.code
                  && OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN == error.domain)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t ockam_channel_deinit(ockam_channel_t* p_ch)
{
  ockam_error_t error = ockam_channel_interface_error_none;

  if (p_ch->channel_reader) ockam_memory_free(gp_ockam_channel_memory, p_ch->channel_reader, sizeof(ockam_reader_t));
  if (p_ch->channel_writer) ockam_memory_free(gp_ockam_channel_memory, p_ch->channel_writer, sizeof(ockam_writer_t));
  if (p_ch->channel_read_buffer)
    ockam_memory_free(gp_ockam_channel_memory, p_ch->channel_read_buffer, MAX_CHANNEL_PACKET_SIZE);
  if (p_ch->channel_write_buffer)
    ockam_memory_free(gp_ockam_channel_memory, p_ch->channel_write_buffer, MAX_CHANNEL_PACKET_SIZE);
  if (p_ch->key.context) ockam_key_deinit(&p_ch->key);

  return error;
}

void dump_route(codec_route_t* route)
{
  printf("dump_route: %d addresses:\n", route->count_addresses);
  for (int i = 0; i < route->count_addresses; ++i) {
    switch (route->p_addresses[i].type) {
    case ADDRESS_LOCAL:
      printf("Local address: %u\n", *(uint16_t*) route->p_addresses[i].address.local_address.address);
      break;
    case ADDRESS_UDP:
      printf("IP: %d.%d.%d.%d:%u\n",
             route->p_addresses[i].address.socket_address.udp_address.host_address.ip_address.ipv4[0],
             route->p_addresses[i].address.socket_address.udp_address.host_address.ip_address.ipv4[1],
             route->p_addresses[i].address.socket_address.udp_address.host_address.ip_address.ipv4[2],
             route->p_addresses[i].address.socket_address.udp_address.host_address.ip_address.ipv4[3],
             route->p_addresses[i].address.socket_address.udp_address.port);
      break;
    default:
      printf("dump_route: address type not implemented\n");
    }
  }
}
