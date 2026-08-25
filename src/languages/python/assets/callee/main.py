from __future__ import annotations

from ffi.main import DEFAULT_SOCKET_PATH, serve_forever
from dispatch import dispatch_message

if __name__ == "__main__":
    serve_forever(DEFAULT_SOCKET_PATH, dispatch_message)
