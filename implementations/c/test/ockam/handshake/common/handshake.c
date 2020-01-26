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

OCKAM_ERR hkdf_dh( uint8_t* hkdf1, uint16_t hkdf1_size, OCKAM_VAULT_KEY_e dh_key, uint8_t*  dh2, uint16_t dh2_size,
                   uint16_t out_size, uint8_t*  out_1, uint8_t*  out_2 )
{
	OCKAM_ERR   status = OCKAM_ERR_NONE;
	uint8_t     pms[KEY_SIZE];
	uint8_t     bytes[2*out_size];

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


OCKAM_ERR encrypt( HANDSHAKE* p_h, uint8_t* p_payload, uint32_t payload_size,
                             uint8_t* p_msg, uint16_t msg_length, uint16_t* p_msg_size )
{
	OCKAM_ERR   status		= OCKAM_ERR_NONE;
	uint8_t     cipher_text[MAX_TRANSMIT_SIZE];
	uint8_t     vector[VECTOR_SIZE];
	uint32_t    offset = 0;

	if( msg_length < (payload_size+TAG_SIZE)) {
		status = OCKAM_ERR_TRANSPORT_BUFFER_TOO_SMALL;
		goto exit_block;
	}

	memset( cipher_text, 0, sizeof(cipher_text) );
	make_vector( p_h->ne, vector );
	status = ockam_vault_aes_gcm_encrypt( p_h->ke, KEY_SIZE, vector, sizeof(vector),
	                                      NULL, 0, &cipher_text[payload_size], TAG_SIZE, p_payload,
	                                      payload_size, cipher_text, payload_size);
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_aes_gcm_encrypt in responder_epilogue_make");
		goto exit_block;
	}
	memcpy( p_msg, cipher_text, TAG_SIZE+payload_size );
	offset += TAG_SIZE+payload_size;
	p_h->ne += 1;
	*p_msg_size = offset;

exit_block:
	return status;
}


OCKAM_ERR decrypt( HANDSHAKE* p_h, uint8_t* p_payload, uint32_t payload_size,
                   uint8_t* p_msg, uint16_t msg_length, uint32_t* p_payload_bytes )
{
	OCKAM_ERR   status		= OCKAM_ERR_NONE;
	uint8_t     uncipher[MAX_TRANSMIT_SIZE];
	uint8_t     tag[TAG_SIZE];
	uint8_t     vector[VECTOR_SIZE];
	uint32_t    offset = 0;
	uint32_t    uncipher_size = 0;

	if( payload_size < (msg_length-TAG_SIZE)) {
		status = OCKAM_ERR_TRANSPORT_BUFFER_TOO_SMALL;
		goto exit_block;
	}

	*p_payload_bytes = msg_length-TAG_SIZE;

	memset( tag, 0, sizeof(tag) );
	memcpy( tag, p_msg+offset+*p_payload_bytes, TAG_SIZE );
	make_vector( p_h->nd, vector );
	memset(uncipher, 0, sizeof(uncipher));
	uncipher_size = msg_length-TAG_SIZE;
	status = ockam_vault_aes_gcm_decrypt( p_h->kd, KEY_SIZE, vector, sizeof(vector),
			NULL, 0, tag, sizeof(tag), p_msg+offset, uncipher_size, uncipher, uncipher_size );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_aes_gcm_decrypt in initiator_m2_recv");
		goto exit_block;
	}
	memcpy( p_payload, uncipher, payload_size );
	p_h->nd += 1;
	print_uint8_str( uncipher, payload_size, "Epilogue:");

exit_block:
	return status;
}
