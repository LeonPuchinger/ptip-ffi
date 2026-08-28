from __future__ import annotations

import unittest

from ffi.socket import MessageSocket
from stream import SocketStream


class MessageSocketTests(unittest.TestCase):
    def test_message_socket_roundtrip(self) -> None:
        left, right = SocketStream.pair()
        try:
            sender = MessageSocket(left)
            receiver = MessageSocket(right)

            sender.send_text("hello")
            self.assertEqual(receiver.receive_text(), "hello")
        finally:
            left.close()
            right.close()

    def test_message_socket_handles_heavy_fragmentation(self) -> None:
        left, right = SocketStream.pair()
        right.max_read_size = 1
        try:
            sender = MessageSocket(left)
            receiver = MessageSocket(right)

            sender.send_text("fragmented message")
            self.assertEqual(receiver.receive_text(), "fragmented message")
        finally:
            left.close()
            right.close()

    def test_message_socket_parses_multiple_messages_from_one_chunk(self) -> None:
        left, right = SocketStream.pair()
        try:
            receiver = MessageSocket(right)
            left.write(b"5:hello,5:world,")

            self.assertEqual(receiver.receive_text(), "hello")
            self.assertEqual(receiver.receive_text(), "world")
        finally:
            left.close()
            right.close()

    def test_message_socket_eof_returns_none(self) -> None:
        left, right = SocketStream.pair()
        try:
            receiver = MessageSocket(right)
            left.close()
            self.assertIsNone(receiver.receive_text())
        finally:
            right.close()

    def test_message_socket_rejects_non_digit_length(self) -> None:
        left, right = SocketStream.pair()
        try:
            receiver = MessageSocket(right)
            left.write(b"x:abc,")

            with self.assertRaisesRegex(ValueError, "Invalid netstring"):
                receiver.receive()
        finally:
            left.close()
            right.close()

    def test_message_socket_rejects_missing_comma(self) -> None:
        left, right = SocketStream.pair()
        try:
            receiver = MessageSocket(right)
            left.write(b"3:abc.")

            with self.assertRaisesRegex(ValueError, "missing comma"):
                receiver.receive()
        finally:
            left.close()
            right.close()


if __name__ == "__main__":
    unittest.main()
