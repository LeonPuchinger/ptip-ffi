from __future__ import annotations

import weakref
from typing import Any
from uuid import uuid4

from .ffi.bridge import CallMessage, DropMessage, ErrorMessage, FunctionTarget, MethodMessage, SendMessage, parameter_to_python, python_to_parameter
from .ffi.main import establish_bridge

def _finalize_reference(uuid: str) -> None:
	bridge = establish_bridge()
	bridge.send(DropMessage(reference=uuid))

{{STUBS}}
