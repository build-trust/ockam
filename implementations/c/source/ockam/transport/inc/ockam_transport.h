#ifndef OCKAM_TRANSPORT_H
#define OCKAM_TRANSPORT_H 1
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/tcp.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#include <netdb.h>
#include <errno.h>
#include <time.h>

/*
 * Transport layer error codes
 */
typedef unsigned long OCKAM_ERROR;
#define			OCKAM_SUCCESS						0
#define			OCKAM_ERR_MALLOC					1
#define			OCKAM_ERR_INIT_TRANSPORT			100
#define			OCKAM_ERR_INVALID_LOCAL_ADDRESS		101
#define			OCKAM_ERR_INVALID_REMOTE_ADDRESS	101
#define			OCKAM_ERR_INIT_SERVER				102
#define			OCKAM_ERR_INVALID_HANDLE			103
#define			OCKAM_ERR_RECEIVER					104
#define			OCKAM_ERR_SENDER					105
#define         OCKAM_ERR_INIT_CLIENT               106

/*
 * Transport layer initialization flags
 */

#define     MAX_HOST_NAME_LENGTH            128
#define     DEFAULT_LISTEN_PORT             8000
#define     MAX_CONNECTIONS                 50

// Opaque to users, this is a pointer to a connection record and is
// cast as such in transport functions.
typedef	void*			OCKAM_CONNECTION_HANDLE;
typedef void*           OCKAM_TCP_SERVER_HANDLE;

// This section should go elsewhere as items are more broadly
// used than just in transport #revisit
//-------------------
// User-friendly IP and DNS addresses
#define	MAX_DNS_NAME_LENGTH		254		// This includes a terminating NULL
#define MAX_DNS_ADDRESS_LENGTH	128		// This includes a terminating NULL

typedef struct {
	char					dns_name[MAX_DNS_NAME_LENGTH];			// "www.name.ext"
	char					ip_address[MAX_DNS_ADDRESS_LENGTH]; 	//"xxx.xxx.xxx.xxx"
} OCKAM_INTERNET_ADDRESS;

typedef	unsigned long	OCKAM_ID;

typedef struct {
	OCKAM_INTERNET_ADDRESS		host_address;
    int                         host_port;
} OCKAM_DEVICE_RECORD;
//------------------

/*
	Transport layer API
 */

OCKAM_ERROR ockam_xp_init_tcp_client( OCKAM_CONNECTION_HANDLE* handle,
    OCKAM_DEVICE_RECORD* p_ockam_device );
OCKAM_ERROR ockam_xp_init_tcp_server( OCKAM_CONNECTION_HANDLE* p_handle,
	OCKAM_DEVICE_RECORD* p_ockam_device );
OCKAM_ERROR ockam_xp_connect();
OCKAM_ERROR ockam_xp_send(OCKAM_CONNECTION_HANDLE handle,
	void* buffer, unsigned int length, unsigned int* p_bytes_sent);
OCKAM_ERROR ockam_xp_receive( OCKAM_CONNECTION_HANDLE handle,
	void* buffer, unsigned int length, unsigned int* p_bytes_received );
OCKAM_ERROR ockam_xp_uninit_client( OCKAM_CONNECTION_HANDLE handle );
OCKAM_ERROR ockam_xp_uninit_server( OCKAM_CONNECTION_HANDLE handle );

#endif
