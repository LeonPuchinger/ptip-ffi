from __future__ import annotations

import socket
from pathlib import Path
from typing import Protocol


class SynchronousStream(Protocol):
    def read(self, buffer: bytearray) -> int | None: ...
    def write(self, buffer: bytes | bytearray | memoryview) -> int: ...
    def close(self) -> None: ...


class SynchronousSocket:
    def __init__(self, sock: socket.socket):
        self.socket = sock

    @classmethod
    def from_path(cls, path: str) -> "SynchronousSocket":
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.connect(path)
        return cls(sock)

    def read(self, buffer: bytearray) -> int | None:
        data = self.socket.recv(len(buffer))
        if not data:
            return None
        buffer[: len(data)] = data
        return len(data)

    def write(self, buffer: bytes | bytearray | memoryview) -> int:
        payload = bytes(buffer)
        self.socket.sendall(payload)
        return len(payload)

    def close(self) -> None:
        self.socket.close()


class SynchronousSocketServer:
    def __init__(self, path: str):
        self.path = path
        socket_path = Path(path)
        if socket_path.exists():
            socket_path.unlink()
        self.server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.server.bind(path)
        self.server.listen(1)

    def accept(self) -> SynchronousSocket:
        sock, _ = self.server.accept()
        return SynchronousSocket(sock)

    def close(self) -> None:
        self.server.close()


class MessageSocket:
    def __init__(self, stream: SynchronousStream):
        self.stream = stream
        self.buffer = bytearray()

    def send(self, data: bytes | bytearray | memoryview) -> None:
        payload = bytes(data)
        header = f"{len(payload)}:".encode("utf-8")
        self.stream.write(header)
        self.stream.write(payload)
        self.stream.write(b",")

    def send_text(self, text: str) -> None:
        self.send(text.encode("utf-8"))

    def receive(self) -> bytes | None:
        while True:
            msg = self._try_parse()
            if msg is not None:
                return msg
            chunk = bytearray(1024)
            read = self.stream.read(chunk)
            if read is None:
                return None
            self.buffer.extend(chunk[:read])

    def receive_text(self) -> str | None:
        message = self.receive()
        if message is None:
            return None
        return message.decode("utf-8")

    def close(self) -> None:
        self.stream.close()

    def _try_parse(self) -> bytes | None:
        colon_index = -1
        for index, byte in enumerate(self.buffer):
            if byte == ord(":"):
                colon_index = index
                break
            if byte < ord("0") or byte > ord("9"):
                raise ValueError("Invalid netstring")
        if colon_index == -1:
            return None

        length = int(self.buffer[:colon_index].decode("ascii"))
        data_start = colon_index + 1
        data_end = data_start + length
        total = data_end + 1
        if len(self.buffer) < total:
            return None
        if self.buffer[data_end] != ord(","):
            raise ValueError("Invalid netstring (missing comma)")
        message = bytes(self.buffer[data_start:data_end])
        del self.buffer[:total]
        return message
