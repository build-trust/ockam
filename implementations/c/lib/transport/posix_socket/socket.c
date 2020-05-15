#include "ockam/syslog.h"
#include "ockam/transport.h"
#include "socket.h"

/**
 * make_socket_address - construct network-friendly address from user-friendly
 * address
 * @param p_ip_address - (in) IP address in "nnn.nnn.nnn.nnn" format
 * @param port - port number, must be non-zero
 * @param p_socketAddress - (out) address converted
 * @return - OCKAM_NO_ERR on success
 */
ockam_error_t make_socket_address(uint8_t* p_ip_address, in_port_t port, struct sockaddr_in* p_socket_address)
{
  ockam_error_t error     = OCKAM_ERROR_NONE;
  int           in_status = 0;

  // Get the host IP address and port
  p_socket_address->sin_family = AF_INET;
  p_socket_address->sin_port   = htons(port);
  if (NULL != p_ip_address) {
    in_status = inet_pton(AF_INET, (char*) p_ip_address, &p_socket_address->sin_addr.s_addr);
    if (1 != in_status) {
      error = TRANSPORT_ERROR_BAD_ADDRESS;
      log_error(error, "inet_pton failed in make_socket_address");
      goto exit;
    }
  } else {
    p_socket_address->sin_addr.s_addr = htonl(INADDR_ANY);
  }

exit:
  return error;
}
