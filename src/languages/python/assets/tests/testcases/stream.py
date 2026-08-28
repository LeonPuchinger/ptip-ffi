from __future__ import annotations

import socket


class SocketStream:
    def __init__(self, sock: socket.socket):
        self.sock = sock
        self.max_read_size = 1024

    @classmethod
    def pair(cls) -> tuple["SocketStream", "SocketStream"]:
        left, right = socket.socketpair()
        return cls(left), cls(right)

    def read(self, buffer: bytearray) -> int | None:
        data = self.sock.recv(min(len(buffer), self.max_read_size))
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