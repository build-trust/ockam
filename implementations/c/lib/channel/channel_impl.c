#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <ctype.h>
#include "ockam/syslog.h"
#include "ockam/memory.h"
#include "ockam/key_agreement.h"
#include "ockam/transport.h"
#include "ockam/io/io_impl.h"
#include "memory/stdlib/stdlib.h"
#include "ockam/channel.h"
#include "channel_impl.h"
#include "ockam/key_agreement.h"
#include "ockam/codec.h"

ockam_error_t channel_read(void*, uint8_t*, size_t, size_t*);
ockam_error_t channel_write(void*, uint8_t*, size_t);

uint8_t clear_text[MAX_CHANNEL_PACKET_SIZE];
uint8_t encoded_text[MAX_CHANNEL_PACKET_SIZE];
uint8_t cipher_text[MAX_CHANNEL_PACKET_SIZE];

ockam_error_t channel_process_message(uint8_t* p_encoded, size_t encoded_text_length,
                                      uint8_t* p_clear_text, size_t* p_clear_text_length)
{
  ockam_error_t error = OCKAM_ERROR_NONE;
  codec_message_type_t message_type = *p_encoded++;
  switch (message_type) {
  case PING:
    break;
  case PAYLOAD:
    *p_clear_text_length = encoded_text_length - sizeof(uint8_t);
    memcpy(p_clear_text, p_encoded, *p_clear_text_length);
    break;
  default:
    error = CHANNEL_ERROR_NOT_IMPLEMENTED;
    goto exit;
  }
exit:
  if(error) log_error(error, __func__ );
  return error;
}

ockam_error_t ockam_channel_init(ockam_channel_t** pp_ch, ockam_channel_attributes_t* p_attrs)
{
  ockam_error_t    error = OCKAM_ERROR_NONE;
  ockam_channel_t* p_ch  = NULL;

  if ((NULL == pp_ch) || (NULL == p_attrs) || (NULL == p_attrs->reader) || (NULL == p_attrs->writer) ||
      (NULL == p_attrs->memory)) {
    error = CHANNEL_ERROR_PARAMS;
    goto exit;
  }

  error = ockam_memory_alloc_zeroed(p_attrs->memory, (void**) &p_ch, sizeof(ockam_channel_t));
  if (error) goto exit;

  p_ch->memory           = p_attrs->memory;

  error = ockam_memory_alloc_zeroed(p_ch->memory, (void**) &p_ch->channel_reader, sizeof(ockam_reader_t));
  if (error) goto exit;
  p_ch->channel_reader->read = channel_read;
  p_ch->channel_reader->ctx  = p_ch;

  error = ockam_memory_alloc_zeroed(p_ch->memory, (void**) &p_ch->channel_writer, sizeof(ockam_writer_t));
  if (error) goto exit;
  p_ch->channel_writer->write = channel_write;
  p_ch->channel_writer->ctx   = p_ch;

  p_ch->transport_reader = p_attrs->reader;
  p_ch->transport_writer = p_attrs->writer;
  p_ch->key.p_reader     = p_ch->channel_reader;
  p_ch->key.p_writer     = p_ch->channel_writer;
  p_ch->key.vault        = p_attrs->vault;

  p_ch->state = CHANNEL_STATE_M1;

  *pp_ch = p_ch;

exit:
  if (error) {
    log_error(error, __func__);
    if (p_ch->channel_reader) ockam_memory_free(p_ch->memory, (void*) p_ch->channel_reader, sizeof(ockam_reader_t));
    if (p_ch->channel_writer) ockam_memory_free(p_ch->memory, (void*) p_ch->channel_writer, sizeof(ockam_writer_t));
    if (p_ch) ockam_memory_free(p_ch->memory, (uint8_t*) p_ch, sizeof(ockam_channel_t));
  }
  return 0;
}

ockam_error_t ockam_channel_connect(ockam_channel_t* p_ch, ockam_reader_t** p_reader, ockam_writer_t** p_writer)
{
  ockam_error_t error = 0;
  error               = ockam_key_establish_initiator_xx(&p_ch->key);
  if (error) goto exit;
  *p_reader                  = p_ch->channel_reader;
  *p_writer                  = p_ch->channel_writer;

exit:
  if (error) log_error(error, __func__ );
  return error;
}

ockam_error_t ockam_channel_accept(ockam_channel_t* p_ch, ockam_reader_t** p_reader, ockam_writer_t** p_writer)
{
  ockam_error_t error = 0;
  error               = ockam_key_establish_responder_xx(&p_ch->key);
  if (error) goto exit;
  *p_reader                  = p_ch->channel_reader;
  *p_writer                  = p_ch->channel_writer;

exit:
  if (error)  log_error(error, __func__);
  return error;
}

ockam_error_t channel_read(void* ctx, uint8_t* clear_text, size_t clear_text_size, size_t* p_clear_text_length)
{
  ockam_error_t error = 0;
  size_t        cipher_text_length = 0;
  size_t        encoded_text_length = 0;
  uint8_t*      p_encoded = encoded_text;

  ockam_channel_t* p_ch = (ockam_channel_t*) ctx;
  error                 = ockam_read(p_ch->transport_reader, cipher_text, sizeof(cipher_text), &cipher_text_length);
  if (error) goto exit;

  if(CHANNEL_STATE_SECURE == p_ch->state) {
    error = xx_decrypt(&p_ch->key, encoded_text, sizeof(encoded_text),
                       cipher_text, cipher_text_length, &encoded_text_length);
    if (error) goto exit;
  } else {
    memcpy(encoded_text, cipher_text, cipher_text_length);
    encoded_text_length = cipher_text_length;
  }

  // Step over header
  p_encoded = decode_ockam_wire(p_encoded);
  if(!p_encoded) { error = CODEC_ERROR_NOT_IMPLEMENTED; goto exit; }
  if(0 != *p_encoded++) { error = CODEC_ERROR_NOT_IMPLEMENTED; goto exit; } //!!onward route
  if(0 != *p_encoded++) { error = CODEC_ERROR_NOT_IMPLEMENTED; goto exit; } //!!return route

  switch(p_ch->state) {
  case CHANNEL_STATE_M1:
    if(REQUEST_CHANNEL != *p_encoded++) { error = CHANNEL_ERROR_KEY_AGREEMENT; goto exit; }
    *p_clear_text_length = encoded_text_length - (p_encoded - encoded_text);
    memcpy(clear_text, p_encoded, *p_clear_text_length);
    p_ch->state = CHANNEL_STATE_M2;
    break;
  case CHANNEL_STATE_M2:
    if(KEY_AGREEMENT_T1_M2 != *p_encoded++) { error = CHANNEL_ERROR_KEY_AGREEMENT; goto exit; }
    *p_clear_text_length = encoded_text_length - (p_encoded - encoded_text);
    memcpy(clear_text, p_encoded, *p_clear_text_length);
    p_ch->state = CHANNEL_STATE_M3;
    break;
  case CHANNEL_STATE_M3:
    if(KEY_AGREEMENT_T1_M3 != *p_encoded++) { error = CHANNEL_ERROR_KEY_AGREEMENT; goto exit; }
    *p_clear_text_length = encoded_text_length - (p_encoded - encoded_text);
    memcpy(clear_text, p_encoded, *p_clear_text_length);
    p_ch->state = CHANNEL_STATE_SECURE;
    break;
  case CHANNEL_STATE_SECURE:
    error = channel_process_message(p_encoded, encoded_text_length, clear_text, p_clear_text_length);
    if(error) goto exit;
    break;
  default:
    error = CHANNEL_ERROR_STATE;
    goto exit;
  }

exit:
  if (error) log_error(error, __func__ );
  return error;
}

ockam_error_t channel_write(void* ctx, uint8_t* clear_text, size_t clear_text_length)
{
  ockam_error_t    error = 0;
  size_t           cipher_text_length = 0;
  size_t           encoded_text_length = 0;
  uint8_t*         p_encoded = encoded_text;
  ockam_channel_t* p_ch = (ockam_channel_t*)ctx;

  p_encoded = encode_ockam_wire(p_encoded);
  if(!p_encoded) { error = OCKAM_ERROR_INTERFACE_CODEC; goto exit; }
  *p_encoded++ = 0; //!! onward route
  *p_encoded++ = 0; //!! return route

  switch(p_ch->state) {
  case CHANNEL_STATE_M1:
    *p_encoded++ = REQUEST_CHANNEL;
    encoded_text_length = p_encoded-encoded_text+clear_text_length;
    cipher_text_length = encoded_text_length;
    memcpy(p_encoded, clear_text, clear_text_length);
    memcpy(cipher_text, encoded_text, encoded_text_length);
    p_ch->state = CHANNEL_STATE_M2;
    break;
  case CHANNEL_STATE_M2:
    *p_encoded++ = KEY_AGREEMENT_T1_M2;
    encoded_text_length = p_encoded-encoded_text+clear_text_length;
    cipher_text_length = encoded_text_length;
    memcpy(p_encoded, clear_text, clear_text_length);
    memcpy(cipher_text, encoded_text, encoded_text_length);
    p_ch->state = CHANNEL_STATE_M3;
    break;
  case CHANNEL_STATE_M3:
    *p_encoded++ = KEY_AGREEMENT_T1_M3;
    encoded_text_length = p_encoded-encoded_text+clear_text_length;
    cipher_text_length = encoded_text_length;
    memcpy(p_encoded, clear_text, clear_text_length);
    memcpy(cipher_text, encoded_text, encoded_text_length);
    p_ch->state = CHANNEL_STATE_SECURE;
    break;
  case CHANNEL_STATE_SECURE:
    *p_encoded++ = PAYLOAD;
    encoded_text_length = p_encoded-encoded_text+clear_text_length;
    memcpy(p_encoded, clear_text, clear_text_length);
    error  = xx_encrypt(&p_ch->key, encoded_text, encoded_text_length,
                        cipher_text, sizeof(cipher_text), &cipher_text_length);
    break;
  default:
    error = CHANNEL_ERROR_NOT_IMPLEMENTED;
    goto exit;
  }

  if (error) goto exit;
  error = ockam_write(p_ch->transport_writer, cipher_text, cipher_text_length);
exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t ockam_channel_deinit(ockam_channel_t* p_ch)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  error = ockam_memory_free(p_ch->memory, p_ch->channel_reader, 0);
  if (error) goto exit;
  error = ockam_memory_free(p_ch->memory, p_ch->channel_writer, 0);
  if (error) goto exit;
  xx_key_deinit(&p_ch->key);
  error = ockam_memory_free(p_ch->memory, p_ch, 0);
exit:
  if (error) log_error(error, __func__);
  return error;
}
