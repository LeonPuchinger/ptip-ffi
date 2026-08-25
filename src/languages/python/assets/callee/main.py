from __future__ import annotations

import os

from ffi.bridge import parse_message
from ffi.socket import MessageSocket, SynchronousSocketServer
from dispatch import dispatch_message

DEFAULT_SOCKET_PATH = os.environ.get("PTIP_FFI_SOCKET_PATH", "/tmp/ptip-ffi-python.sock")


def serve_forever(socket_path: str, handler) -> None:
    server = SynchronousSocketServer(socket_path)
    try:
        while True:
            connection = server.accept()
            try:
                message_socket = MessageSocket(connection)
                while True:
                    message_text = message_socket.receive_text()
                    if message_text is None:
                        break
                    response = handler(parse_message(message_text))
                    if response is not None:
                        message_socket.send_text(response.serialize())
            finally:
                connection.close()
    finally:
        server.close()


if __name__ == "__main__":
    serve_forever(DEFAULT_SOCKET_PATH, dispatch_message)
