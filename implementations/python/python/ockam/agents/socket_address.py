import socket


def parse_host_and_port(address: str | int, default_host: str = "127.0.0.1") -> tuple[str, int]:
    """
    Parse a socket address and return the host and port.
    If the address is just a port number, return the default host with that port.
    """
    host = default_host
    port = None
    if isinstance(address, int):
        port = address
    elif isinstance(address, str) and ":" in address:
        _host, port_str = address.split(":", 1)
        if _host:
            host = _host
        port = int(port_str)

    is_valid_address_guard(host, port)
    return host, port


def is_valid_address_guard(host: str, port: int):
    """
    Raises ValueError if the address is invalid.
    """
    if port < 0 or port > 65535:
        raise ValueError(f"Port {port} is out of range (0-65535)")
    try:
        socket.getaddrinfo(host, port)  # Check if host resolves
    except socket.gaierror:
        raise ValueError(f"Host {host} is not a valid address")


def is_port_free(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        try:
            s.bind(("", port))
            return True
        except OSError:
            return False


def get_free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("", 0))
        return s.getsockname()[1]
