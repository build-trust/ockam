#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <ctype.h>
#include "ockam/syslog.h"
#include "ockam/memory.h"
#include "ockam/key_agreement.h"
#include "ockam/key_agreement/xx.h"
#include "ockam/transport.h"
#include "ockam/io/impl.h"
#include "ockam/channel.h"
#include "channel_impl.h"
#include "ockam/codec.h"

ockam_memory_t* gp_ockam_channel_memory = NULL;

ockam_error_t channel_read(void*, uint8_t*, size_t, size_t*);
ockam_error_t channel_write(void*, uint8_t*, size_t);

uint8_t g_encoded_text[MAX_CHANNEL_PACKET_SIZE];
uint8_t g_cipher_text[MAX_CHANNEL_PACKET_SIZE];

uint8_t* channel_encode_header(ockam_channel_t* p_ch, uint8_t* p_encoded)
{
  p_encoded = encode_ockam_wire(p_encoded);
  if (NULL == p_encoded) goto exit;
  *p_encoded++ = 0; //!! onward route not implemented
  *p_encoded++ = 0; //!! return route not implemented
exit:
  return p_encoded;
}

uint8_t* channel_deocde_header(ockam_channel_t* p_ch, uint8_t* p_encoded)
{
  p_encoded = decode_ockam_wire(p_encoded);
  if (NULL == p_encoded) goto exit;
  if (!p_encoded) {
    p_encoded = NULL;
    goto exit;
  }
  if (0 != *p_encoded++) {
    p_encoded = NULL;
    goto exit;
  } //!!onward route
  if (0 != *p_encoded++) {
    p_encoded = NULL;
    goto exit;
  } //!!return route
exit:
  return p_encoded;
}

ockam_error_t channel_decrypt(ockam_channel_t* p_ch,
                              uint8_t*         p_cipher_text,
                              size_t           cipher_text_length,
                              uint8_t*         p_encoded_text,
                              size_t           encoded_text_size,
                              size_t*          p_encoded_text_length)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if (CHANNEL_STATE_SECURE == p_ch->state) {
    error = ockam_key_decrypt(
      &p_ch->key, p_encoded_text, encoded_text_size, p_cipher_text, cipher_text_length, p_encoded_text_length);
    if (error) goto exit;
  } else {
    ockam_memory_copy(gp_ockam_channel_memory, p_encoded_text, p_cipher_text, cipher_text_length);
    *p_encoded_text_length = cipher_text_length;
  }
exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t channel_process_message(uint8_t* p_encoded,
                                      size_t   encoded_text_length,
                                      uint8_t* p_clear_text,
                                      size_t*  p_clear_text_length)
{
  ockam_error_t        error        = OCKAM_ERROR_NONE;
  codec_message_type_t message_type = *p_encoded++;
  switch (message_type) {
  case PING:
    break;
  case PAYLOAD:
    *p_clear_text_length = encoded_text_length - sizeof(uint8_t);
    ockam_memory_copy(gp_ockam_channel_memory, p_clear_text, p_encoded, *p_clear_text_length);
    break;
  default:
    error = CHANNEL_ERROR_NOT_IMPLEMENTED;
    goto exit;
  }
exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t ockam_channel_init(ockam_channel_t* p_ch, ockam_channel_attributes_t* p_attrs)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((NULL == p_ch) || (NULL == p_attrs) || (NULL == p_attrs->reader) || (NULL == p_attrs->writer) ||
      (NULL == p_attrs->memory)) {
    error = CHANNEL_ERROR_PARAMS;
    goto exit;
  }

  gp_ockam_channel_memory = p_attrs->memory;
  p_ch->vault             = p_attrs->vault;

  error = ockam_memory_alloc_zeroed(gp_ockam_channel_memory, (void**) &p_ch->channel_reader, sizeof(ockam_reader_t));
  if (error) goto exit;
  p_ch->channel_reader->read = channel_read;
  p_ch->channel_reader->ctx  = p_ch;

  error = ockam_memory_alloc_zeroed(gp_ockam_channel_memory, (void**) &p_ch->channel_writer, sizeof(ockam_writer_t));
  if (error) goto exit;
  p_ch->channel_writer->write = channel_write;
  p_ch->channel_writer->ctx   = p_ch;

  p_ch->transport_reader = p_attrs->reader;
  p_ch->transport_writer = p_attrs->writer;

  error = ockam_xx_key_initialize(
    &p_ch->key, gp_ockam_channel_memory, p_ch->vault, p_ch->channel_reader, p_ch->channel_writer);

  p_ch->state = CHANNEL_STATE_M1;

exit:
  if (error) {
    log_error(error, __func__);
    if (p_ch) {
      if (p_ch->channel_reader)
        ockam_memory_free(gp_ockam_channel_memory, (void*) p_ch->channel_reader, sizeof(ockam_reader_t));
      if (p_ch->channel_writer)
        ockam_memory_free(gp_ockam_channel_memory, (void*) p_ch->channel_writer, sizeof(ockam_writer_t));
    }
  }
  return 0;
}

ockam_error_t ockam_channel_connect(ockam_channel_t* p_ch, ockam_reader_t** p_reader, ockam_writer_t** p_writer)
{
  ockam_error_t error = 0;

  error = ockam_key_initiate(&p_ch->key);
  if (error) goto exit;
  *p_reader = p_ch->channel_reader;
  *p_writer = p_ch->channel_writer;

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t ockam_channel_accept(ockam_channel_t* p_ch, ockam_reader_t** p_reader, ockam_writer_t** p_writer)
{
  ockam_error_t error = 0;
  error               = ockam_key_respond(&p_ch->key);
  if (error) goto exit;
  *p_reader = p_ch->channel_reader;
  *p_writer = p_ch->channel_writer;

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t channel_read(void* ctx, uint8_t* p_clear_text, size_t clear_text_size, size_t* p_clear_text_length)
{
  ockam_error_t    error               = 0;
  size_t           cipher_text_length  = 0;
  size_t           encoded_text_length = 0;
  uint8_t*         p_encoded           = g_encoded_text;
  ockam_channel_t* p_ch                = (ockam_channel_t*) ctx;

  error = ockam_read(p_ch->transport_reader, g_cipher_text, sizeof(g_cipher_text), &cipher_text_length);
  if (error) goto exit;

  error = channel_decrypt(
    p_ch, g_cipher_text, cipher_text_length, g_encoded_text, sizeof(g_encoded_text), &encoded_text_length);
  if (error) goto exit;

  p_encoded = channel_deocde_header(p_ch, p_encoded);
  if (NULL == p_encoded) {
    error = CHANNEL_ERROR_NOT_IMPLEMENTED;
    goto exit;
  }

  if (CHANNEL_STATE_SECURE == p_ch->state) {
    error = channel_process_message(p_encoded, encoded_text_length, p_clear_text, p_clear_text_length);
    if (error) goto exit;
  } else {
    codec_message_type_t message_type = *p_encoded++;
    *p_clear_text_length              = encoded_text_length - (p_encoded - g_encoded_text);
    ockam_memory_copy(gp_ockam_channel_memory, p_clear_text, p_encoded, *p_clear_text_length);
    switch (p_ch->state) {
    case CHANNEL_STATE_M1:
      if (REQUEST_CHANNEL != message_type) {
        error = CHANNEL_ERROR_KEY_AGREEMENT;
        goto exit;
      }
      p_ch->state = CHANNEL_STATE_M2;
      break;
    case CHANNEL_STATE_M2:
      if (KEY_AGREEMENT_T1_M2 != message_type) {
        error = CHANNEL_ERROR_KEY_AGREEMENT;
        goto exit;
      }
      p_ch->state = CHANNEL_STATE_M3;
      break;
    case CHANNEL_STATE_M3:
      if (KEY_AGREEMENT_T1_M3 != message_type) {
        error = CHANNEL_ERROR_KEY_AGREEMENT;
        goto exit;
      }
      p_ch->state = CHANNEL_STATE_SECURE;
      break;
    default:
      error = CHANNEL_ERROR_STATE;
      goto exit;
    }
  }

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t channel_write(void* ctx, uint8_t* p_clear_text, size_t clear_text_length)
{
  ockam_error_t    error               = 0;
  size_t           cipher_text_length  = 0;
  size_t           encoded_text_length = 0;
  uint8_t*         p_encoded           = g_encoded_text;
  ockam_channel_t* p_ch                = (ockam_channel_t*) ctx;

  p_encoded = channel_encode_header(p_ch, p_encoded);
  if (!p_encoded) {
    error = CHANNEL_ERROR_NOT_IMPLEMENTED;
    goto exit;
  }

  if (CHANNEL_STATE_SECURE == p_ch->state) {
    *p_encoded++        = PAYLOAD;
    encoded_text_length = p_encoded - g_encoded_text + clear_text_length;
    ockam_memory_copy(gp_ockam_channel_memory, p_encoded, p_clear_text, clear_text_length);
    error = ockam_key_encrypt(
      &p_ch->key, g_encoded_text, encoded_text_length, g_cipher_text, sizeof(g_cipher_text), &cipher_text_length);
    if (error) goto exit;
  } else {
    switch (p_ch->state) {
    case CHANNEL_STATE_M1:
      *p_encoded++ = REQUEST_CHANNEL;
      p_ch->state  = CHANNEL_STATE_M2;
      break;
    case CHANNEL_STATE_M2:
      *p_encoded++ = KEY_AGREEMENT_T1_M2;
      p_ch->state  = CHANNEL_STATE_M3;
      break;
    case CHANNEL_STATE_M3:
      *p_encoded++ = KEY_AGREEMENT_T1_M3;
      p_ch->state  = CHANNEL_STATE_SECURE;
      break;
    case CHANNEL_STATE_SECURE:
      break;
    default:
      error = CHANNEL_ERROR_NOT_IMPLEMENTED;
      goto exit;
    }
    encoded_text_length = p_encoded - g_encoded_text + clear_text_length;
    cipher_text_length  = encoded_text_length;
    ockam_memory_copy(gp_ockam_channel_memory, p_encoded, p_clear_text, clear_text_length);
    ockam_memory_copy(gp_ockam_channel_memory, g_cipher_text, g_encoded_text, encoded_text_length);
  }

  error = ockam_write(p_ch->transport_writer, g_cipher_text, cipher_text_length);
  if (error) goto exit;

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t ockam_channel_deinit(ockam_channel_t* p_ch)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  error = ockam_memory_free(gp_ockam_channel_memory, p_ch->channel_reader, 0);
  if (error) goto exit;
  error = ockam_memory_free(gp_ockam_channel_memory, p_ch->channel_writer, 0);
  if (error) goto exit;
  ockam_key_deinit(&p_ch->key);
exit:
  if (error) log_error(error, __func__);
  return error;
}
