#ifndef TEST_TCP_H
#define TEST_TCP_H

OckamError file_compare(char *p_f1, char *p_f2);
int testTcpServer(OckamInternetAddress *pHostAddress);
int testTcpClient(OckamInternetAddress *pHostAddress);

#endif