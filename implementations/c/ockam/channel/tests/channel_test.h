#include <stdlib.h>
#include "ockam/channel.h"
#include "ockam/vault.h"

#define PING_TEXT "PING_TEXT"
#define PING_SIZE 10
#define ACK_TEXT  "ACK_TEXT"
#define ACK_SIZE  9

#define MAX_XX_TRANSMIT_SIZE 2048

ockam_error_t channel_initiator(ockam_vault_t*   vault,
                                ockam_memory_t*  p_memory,
                                codec_address_t* local_host_address,
                                codec_address_t* local_address,
                                codec_route_t*   route);
ockam_error_t channel_responder(ockam_vault_t*   vault,
                                ockam_memory_t*  p_memory,
                                codec_address_t* local_host_address,
                                codec_address_t* local_address);
ockam_error_t
read_route_configuration(char* filename, codec_route_t* route, codec_address_t* initiator, codec_address_t* responder);
