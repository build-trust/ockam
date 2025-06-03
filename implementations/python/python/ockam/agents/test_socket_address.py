import socket
import pytest
from .socket_address import parse_host_and_port, is_valid_address_guard, is_port_free, get_free_port


class TestParseHostAndPort:
    def test_integer_port(self):
        host, port = parse_host_and_port(8000)
        assert host == "127.0.0.1"
        assert port == 8000

    def test_host_and_port(self):
        host, port = parse_host_and_port("192.168.1.1:8080")
        assert host == "192.168.1.1"
        assert port == 8080

    def test_port_only(self):
        host, port = parse_host_and_port(":9000")
        assert host == "127.0.0.1"
        assert port == 9000

    def test_empty_host(self):
        host, port = parse_host_and_port(":8888")
        assert host == "127.0.0.1"
        assert port == 8888

    def test_invalid_address(self):
        with pytest.raises(ValueError):
            parse_host_and_port("invalid:not_a_port")


class TestIsValidAddressGuard:
    def test_valid_address(self):
        is_valid_address_guard("127.0.0.1", 8000)

    def test_port_out_of_range_low(self):
        with pytest.raises(ValueError, match="Port -1 is out of range"):
            is_valid_address_guard("127.0.0.1", -1)

    def test_port_out_of_range_high(self):
        with pytest.raises(ValueError, match="Port 65536 is out of range"):
            is_valid_address_guard("127.0.0.1", 65536)

    def test_invalid_host(self):
        with pytest.raises(ValueError, match="Host invalid_host_name is not a valid address"):
            is_valid_address_guard("invalid_host_name", 8000)


class TestIsPortFree:
    def test_free_port(self):
        with socket.socket() as s:
            s.bind(("", 0))
            port = s.getsockname()[1]

        assert is_port_free(port) is True

    def test_occupied_port(self):
        s = socket.socket()
        try:
            s.bind(("", 0))
            port = s.getsockname()[1]
            assert is_port_free(port) is False
        finally:
            s.close()


class TestGetFreePort:
    def test_get_free_port(self):
        port = get_free_port()
        assert is_port_free(port) is True
        s = socket.socket()
        try:
            s.bind(("", port))
            assert True
        except OSError:
            assert False, f"Port {port} returned by get_free_port() was not free"
        finally:
            s.close()
