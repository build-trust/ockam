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

ockam_error_t channel_read(void*, uint8_t*, size_t, size_t*);
ockam_error_t channel_write(void*, uint8_t*, size_t);

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
  p_ch->transport_reader = p_attrs->reader;
  p_ch->transport_writer = p_attrs->writer;
  p_ch->key.p_reader     = p_attrs->reader;
  p_ch->key.p_writer     = p_attrs->writer;
  p_ch->key.vault        = p_attrs->vault;

  *pp_ch = p_ch;

exit:
  if (error) {
    log_error(error, __func__);
    if (p_ch) ockam_memory_free(p_ch->memory, (uint8_t*) p_ch, sizeof(ockam_channel_t));
  }
  return 0;
}

ockam_error_t ockam_channel_connect(ockam_channel_t* p_ch, ockam_reader_t** p_reader, ockam_writer_t** p_writer)
{
  ockam_error_t error = 0;
  error               = ockam_key_establish_initiator_xx(&p_ch->key);
  if (error) goto exit;
  error = ockam_memory_alloc_zeroed(p_ch->memory, (void**) &p_ch->channel_reader, sizeof(ockam_reader_t));
  if (error) goto exit;
  p_ch->channel_reader->read = channel_read;
  p_ch->channel_reader->ctx  = p_ch;
  *p_reader                  = p_ch->channel_reader;

  error = ockam_memory_alloc_zeroed(p_ch->memory, (void**) &p_ch->channel_writer, sizeof(ockam_writer_t));
  if (error) goto exit;
  p_ch->channel_writer->write = channel_write;
  p_ch->channel_writer->ctx   = p_ch;
  *p_writer                   = p_ch->channel_writer;

exit:
  if (error) {
    log_error(error, __func__);
    if (p_ch->channel_reader) ockam_memory_free(p_ch->memory, (void*) p_ch->channel_reader, sizeof(ockam_reader_t));
  }
  return error;
}

ockam_error_t ockam_channel_accept(ockam_channel_t* p_ch, ockam_reader_t** p_reader, ockam_writer_t** p_writer)
{
  ockam_error_t error = 0;
  error               = ockam_key_establish_responder_xx(&p_ch->key);
  if (error) goto exit;
  error = ockam_memory_alloc_zeroed(p_ch->memory, (void**) &p_ch->channel_reader, sizeof(ockam_reader_t));
  if (error) goto exit;
  p_ch->channel_reader->read = channel_read;
  p_ch->channel_reader->ctx  = p_ch;
  *p_reader                  = p_ch->channel_reader;

  error = ockam_memory_alloc_zeroed(p_ch->memory, (void**) &p_ch->channel_writer, sizeof(ockam_writer_t));
  if (error) goto exit;
  p_ch->channel_writer->write = channel_write;
  p_ch->channel_writer->ctx   = p_ch;
  *p_writer                   = p_ch->channel_writer;

exit:
  if (error) {
    log_error(error, __func__);
    if (p_ch->channel_reader) ockam_memory_free(p_ch->memory, (void*) p_ch->channel_reader, sizeof(ockam_reader_t));
  }
  return error;
}

ockam_error_t channel_read(void* ctx, uint8_t* clear_text, size_t clear_text_size, size_t* clear_text_length)
{
  ockam_error_t error = 0;
  uint8_t       cipher_text[1024]; //!!
  size_t        cipher_text_length;

  ockam_channel_t* p_ch = (ockam_channel_t*) ctx;
  error                 = ockam_read(p_ch->transport_reader, cipher_text, 1024, &cipher_text_length);
  if (error) goto exit;
  error = xx_decrypt(&p_ch->key, clear_text, clear_text_size, cipher_text, cipher_text_length, clear_text_length);
exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t channel_write(void* ctx, uint8_t* clear_text, size_t clear_text_length)
{
  ockam_error_t    error = 0;
  uint8_t          cipher_text[1024]; //!!
  size_t           cipher_text_length;
  ockam_channel_t* p_ch = (ockam_channel_t*) ctx;
  error                 = xx_encrypt(&p_ch->key, clear_text, clear_text_length, cipher_text, 1024, &cipher_text_length);
  if (error) goto exit;
  error = ockam_write(p_ch->transport_writer, cipher_text, cipher_text_length);
exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t ockam_channel_deinit(ockam_channel_t* p_ch)
{
  ockam_error_t error = OCKAM_ERROR_NONE;
  error               = ockam_memory_free(p_ch->memory, p_ch->channel_reader, 0);
  if (error) goto exit;
  error = ockam_memory_free(p_ch->memory, p_ch->channel_writer, 0);
  if (error) goto exit;
  error = ockam_memory_free(p_ch->memory, p_ch, 0);
  xx_key_deinit(&p_ch->key);
exit:
  if (error) log_error(error, __func__);
  return error;
}
