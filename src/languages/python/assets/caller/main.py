from __future__ import annotations

import os
import subprocess

from .bridge import Bridge
from .socket import MessageSocket, SynchronousSocket

_internal_bridge: Bridge | None = None
_internal_library: subprocess.Popen[str] | None = None


def establish_bridge() -> Bridge:
    global _internal_bridge
    global _internal_library

    if _internal_bridge is not None:
        return _internal_bridge

    invoke = os.environ.get("FFI_LIBRARY_INVOKE")
    if not invoke or invoke.strip() == "":
        raise RuntimeError("FFI_LIBRARY_INVOKE is not set")

    # TODO: provide lifecycle management to terminate the library process
    _internal_library = subprocess.Popen(
        ["/bin/sh", "-c", invoke],
        stdout=subprocess.PIPE,
        stderr=None,
        text=True,
        bufsize=1,
    )
    assert _internal_library.stdout is not None

    stdout_text = ""
    while True:
        line = _internal_library.stdout.readline()
        if line == "":
            code = _internal_library.poll()
            raise RuntimeError(f"Library process exited before printing the socket path: {code}")
        stdout_text = f"{stdout_text}{line}"
        if "\n" in stdout_text:
            break

    socket_path = ""
    for value in reversed(stdout_text.strip().splitlines()):
        candidate = value.strip()
        if candidate:
            socket_path = candidate
            break

    if not socket_path:
        raise RuntimeError("Library process did not print a socket path to stdout")

    stream_socket = SynchronousSocket.from_path(socket_path)
    datagram_socket = MessageSocket(stream_socket)
    _internal_bridge = Bridge(datagram_socket)
    return _internal_bridge


def establishBridge() -> Bridge:
    return establish_bridge()
