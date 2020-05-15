#ifndef TEST_TCP_H
#define TEST_TCP_H
#include "ockam/error.h"
#include "ockam/transport.h"

ockam_error_t file_compare(char* p_f1, char* p_f2);
int           test_tcp_server(ockam_ip_address_t*, char*);
int           test_tcp_client(ockam_ip_address_t*, char*);

#endif
