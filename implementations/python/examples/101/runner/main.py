from sys import argv

from ockam import Node, HttpServer

Node.start(http_server=HttpServer(listen_address=argv[1]))
