from __future__ import annotations

import os
from typing import Any, Callable

from .bridge import Message, parse_message
from .socket import MessageSocket, SynchronousSocket, SynchronousSocketServer

DEFAULT_SOCKET_PATH = os.environ.get("PTIP_FFI_SOCKET_PATH", "/tmp/ptip-ffi-python.sock")


def socket_path_from_environment(default: str | None = None) -> str:
    return os.environ.get("PTIP_FFI_SOCKET_PATH", default or DEFAULT_SOCKET_PATH)


def open_client(socket_path: str | None = None) -> MessageSocket:
    return MessageSocket(SynchronousSocket.from_path(socket_path or socket_path_from_environment()))


def exchange(message: Message, socket_path: str | None = None) -> Any:
    connection = open_client(socket_path)
    try:
        connection.send_text(message.serialize())
        response_text = connection.receive_text()
        if response_text is None:
            raise RuntimeError("Callee closed the connection without responding")
        return parse_message(response_text)
    finally:
        connection.close()


def serve_forever(socket_path: str, handler: Callable[[Message], Any]) -> None:
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
