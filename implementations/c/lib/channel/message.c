//#include <stdlib.h>
//#include <stdio.h>
//#include <string.h>
//#include <ctype.h>
//#include "ockam/syslog.h"
//#include "ockam/memory.h"
//#include "ockam/key_agreement.h"
//#include "key_agreement/xx/xx.h"
//#include "ockam/transport.h"
//#include "io/io_impl.h"
//#include "ockam/channel.h"
//#include "channel_impl.h"
//#include "ockam/codec.h"
//
// ockam_error_t process_message (ockam_channel_t* p_ch, uint8_t** pp_encoded, uint8_t* local_address) {
//  ockam_error_t error = 0;
//  uint8_t* p_encoded = *pp_encoded;
//  uint16_t version = 0;
//  uint8_t address_count = 0;
//
//  /// If onward route number of addresses is 0, this is an unencrypted message to be handled at the
//  /// channel transport level (ie here). If non-zero, there must be exactly one local address, as this
//  /// implementation does not perform routing.
//
//  // For now the only supported version is V1
//  p_encoded = decode_ockam_wire(p_encoded, &version);
//  if((!p_encoded) || (1 != version)){
//    error = CHANNEL_ERROR_NOT_IMPLEMENTED;
//    goto exit;
//  }
//
//  address_count = *p_encoded++;
//  switch (address_count) {
//  case 0:
//    // handle system level stuff
//    error = process_system_message(p_ch, p_encoded);
//    break;
//  case 1:
//    // decode local address and return for further processing
//    break;
//  default:
//    error = CHANNEL_ERROR_MESSAGE;
//    break;
//  }
//
// exit:
//  if(error) log_error(error, __func__ );
//  return error;
//};
//
