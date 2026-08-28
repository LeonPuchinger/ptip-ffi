from __future__ import annotations

import os
from uuid import uuid4

from ffi.bridge import parse_message
from ffi.socket import MessageSocket, SynchronousSocketServer
from dispatch import dispatch_message

DEFAULT_SOCKET_PATH = os.environ.get("PTIP_FFI_SOCKET_PATH", f"/tmp/ptip-ffi-socket-{uuid4().hex}.sock")


def serve_forever(server: SynchronousSocketServer, handler) -> None:
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


if __name__ == "__main__":
    server = SynchronousSocketServer(DEFAULT_SOCKET_PATH)
    try:
        print(DEFAULT_SOCKET_PATH, flush=True)
        serve_forever(server, dispatch_message)
    finally:
        server.close()
