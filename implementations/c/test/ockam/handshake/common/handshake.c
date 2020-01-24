#include <stdlib.h>
#include <string.h>
#include "ockam/vault.h"
#include "ockam/error.h"
#include "ockam/syslog.h"
#include "handshake.h"

void print_uint8_str( uint8_t* p, uint16_t size, char* msg )
{
	printf("\n%s %d bytes: \n", msg, size);
	for( int i = 0; i < size; ++i ) printf( "%0.2x", *p++ );
	printf("\n");
}

uint32_t hkdf_dh_count = 0;

OCKAM_ERR hkdf_dh( uint8_t* hkdf1, uint16_t hkdf1_size, OCKAM_VAULT_KEY_e dh_key, uint8_t*  dh2, uint16_t dh2_size,
                   uint16_t out_size, uint8_t*  out_1, uint8_t*  out_2 )
{
	OCKAM_ERR   status = OCKAM_ERR_NONE;
	uint8_t     pms[KEY_SIZE];
	uint8_t     bytes[2*out_size];

	printf("\n\nhkdf_dh %d", hkdf_dh_count++);
	print_uint8_str( hkdf1, hkdf1_size, "encrypting with");
	printf("secret between %d and :", dh_key);
	print_uint8_str( dh2, dh2_size, "" );

	status = ockam_vault_ecdh( dh_key, dh2, dh2_size, pms, KEY_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_ecdh in responder_m2_send");
		goto exit_block;
	}

	// ck, k = HKDF( ck, pms )
	status = ockam_vault_hkdf( pms, KEY_SIZE, hkdf1, hkdf1_size, NULL, 0, bytes, sizeof(bytes));
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_hkdf in responder_m2_send");
		goto exit_block;
	}
	memcpy( out_1, bytes, out_size );
	memcpy( out_2, bytes, out_size );

exit_block:
	return status;
}

void string_to_hex(char* hexstring, uint8_t* val, uint32_t* p_bytes )
{
	const char* pos = hexstring;

	*p_bytes = 0;
	/* !! no sanitization or error-checking whatsoever */
	for (size_t count = 0; count < (strlen(hexstring)/2); count++) {
		sscanf(pos, "%2hhx", &val[count]);
		pos += 2;
		(*p_bytes)++;
	}
}

uint32_t hashcount = 0;

OCKAM_ERR mix_hash( HANDSHAKE* p_handshake,  uint8_t* p_bytes, uint16_t b_length )
{
	OCKAM_ERR       status = OCKAM_ERR_NONE;
	uint8_t*        p_h = &p_handshake->h[0];
	uint8_t         string[MAX_TRANSMIT_SIZE];
	uint8_t         hash[SHA256_SIZE];

	memset( &hash[0], 0, sizeof(hash) );
	memset( &string[0], 0, sizeof(string) );
	memcpy( &string[0], &p_h[0], SHA256_SIZE );
	memcpy( &string[SHA256_SIZE], p_bytes, b_length );
	status = ockam_vault_sha256( (uint8_t *)&string[0], SHA256_SIZE+b_length, (uint8_t *)&hash[0], SHA256_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_sha256 in mix_hash");
		goto exit_block;
	}
	memcpy( p_h, hash, SHA256_SIZE );

	printf("Hash %d: ", hashcount++ );
	print_uint8_str( hash, SHA256_SIZE, "" );

exit_block:
	return status;
}

OCKAM_ERR make_vector( uint64_t nonce, uint8_t* p_vector )
{
	uint8_t*    pv;
	uint8_t*    pn = (uint8_t*)&nonce;

	memset( p_vector, 0, VECTOR_SIZE );
	pv = p_vector+4;
	pn += 7;
	for( int i = 7; i >= 0; --i ) {
		*pv++ = *pn--;
	}
	return OCKAM_ERR_NONE;
}
