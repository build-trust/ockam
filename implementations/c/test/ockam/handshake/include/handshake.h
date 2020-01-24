#include <stdlib.h>

#define KEY_SIZE 32
#define NAME_SIZE 28
#define SHA256_SIZE 32
#define NAME "Noise_XX_25519_AESGCM_SHA256"
#define MAX_TRANSMIT_SIZE 2048
#define DHLEN 32
#define TAG_SIZE 16
#define VECTOR_SIZE 12
#define EPI_STRING_SIZE 30
#define EPI_BYTE_SIZE 15
#define EPI_INITIATOR "7375626d6172696e6579656c6c6f77"
#define EPI_RESPONDER "79656c6c6f777375626d6172696e65"

typedef struct  {

	uint64_t    nonce;
	uint8_t     s[KEY_SIZE];
	uint8_t     rs[KEY_SIZE];
	uint8_t     e[KEY_SIZE];
	uint8_t     re[KEY_SIZE];
	uint8_t     k[KEY_SIZE];
	uint8_t     ck[SHA256_SIZE];
	uint8_t     h[SHA256_SIZE];
	uint8_t     ke[KEY_SIZE];
	uint8_t     kd[KEY_SIZE];
	uint64_t    ne;
	uint64_t    nd;
} HANDSHAKE;

OCKAM_ERR mix_hash( HANDSHAKE* p_handshake,  uint8_t* p_bytes, uint16_t b_length );

OCKAM_ERR hkdf_dh( uint8_t* hkdf1, uint16_t hkdf1_size, OCKAM_VAULT_KEY_e dh_key, uint8_t*  dh2, uint16_t dh2_size,
                   uint16_t out_size, uint8_t*  out_1, uint8_t*  out_2 );

OCKAM_ERR encrypt_tag( HANDSHAKE* p_h, uint8_t* p_key, uint16_t key_size, uint8_t* p_nonce, uint16_t nonce_size,
                       uint8_t* p_in,  uint32_t size_in, uint8_t* p_out, uint32_t* p_size_out );

OCKAM_ERR make_vector( uint64_t nonce, uint8_t* p_vector );

void print_uint8_str( uint8_t* p, uint16_t size, char* msg );

void string_to_hex(char* hexstring, uint8_t* val, uint32_t* p_bytes );
