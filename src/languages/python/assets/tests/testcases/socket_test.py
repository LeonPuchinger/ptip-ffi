from __future__ import annotations

import socket
import unittest

from ffi.socket import MessageSocket


class SocketStream:
    def __init__(self, sock: socket.socket):
        self.sock = sock

    def read(self, buffer: bytearray) -> int | None:
        data = self.sock.recv(len(buffer))
        if not data:
            return None
        buffer[: len(data)] = data
        return len(data)

    def write(self, buffer: bytes | bytearray | memoryview) -> int:
        payload = bytes(buffer)
        self.sock.sendall(payload)
        return len(payload)

    def close(self) -> None:
        self.sock.close()


class MessageSocketTests(unittest.TestCase):
    def test_message_socket_roundtrip(self) -> None:
        left, right = socket.socketpair()
        try:
            sender = MessageSocket(SocketStream(left))
            receiver = MessageSocket(SocketStream(right))

            sender.send_text("hello")
            self.assertEqual(receiver.receive_text(), "hello")
        finally:
            left.close()
            right.close()


if __name__ == "__main__":
    unittest.main()
