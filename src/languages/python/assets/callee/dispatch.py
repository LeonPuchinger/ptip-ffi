from __future__ import annotations

import importlib
import uuid as uuidlib
from typing import Any

from ffi.bridge import (
    AcknowledgeMessage,
    CallMessage,
    DropMessage,
    ErrorMessage,
    MethodMessage,
    Parameter,
    RequestMessage,
    SendMessage,
    UpdateMessage,
    parameter_to_python,
    python_to_parameter,
)
_LIBRARY = importlib.import_module("library.index")
_INSTANCE_REGISTRY: dict[str, Any] = {}

def _store_instance(value: Any) -> str:
    reference = str(uuidlib.uuid4())
    _INSTANCE_REGISTRY[reference] = value
    return reference


def _store_instance_with_reference(reference: str, value: Any) -> str:
    _INSTANCE_REGISTRY[reference] = value
    return reference

def _result_to_parameter(result: Any, reference: str | None = None) -> Parameter:
    if result is None:
        return Parameter(kind="string", value="undefined")
    if isinstance(result, Parameter):
        return result
    if reference is not None:
        return Parameter(kind="reference", value=_store_instance_with_reference(reference, result))
    if isinstance(result, bool):
        return Parameter(kind="boolean", value=result)
    if isinstance(result, int) and not isinstance(result, bool):
        return Parameter(kind="integer", value=result)
    if isinstance(result, float):
        return Parameter(kind="float", value=result)
    if isinstance(result, str):
        return Parameter(kind="string", value=result)
    if hasattr(result, "uuid"):
        instance_reference = str(getattr(result, "uuid"))
        _INSTANCE_REGISTRY[instance_reference] = result
        return Parameter(kind="reference", value=instance_reference)
    return Parameter(kind="reference", value=_store_instance(result))


def _is_constructor_call(callee: Any, target: Any) -> bool:
    return hasattr(callee, "name") and isinstance(target, type)

def _resolve_target(callee: Any) -> Any:
    if hasattr(callee, "name"):
        return getattr(_LIBRARY, callee.name)
    if hasattr(callee, "type_name") and hasattr(callee, "method_name"):
        return getattr(getattr(_LIBRARY, callee.type_name), callee.method_name)
    raise TypeError(f"Unsupported call target: {type(callee)!r}")

def _invoke_target(
    target: Any,
    positional_parameters: list[Parameter],
    named_parameters: dict[str, Parameter],
    reference: str | None = None,
) -> Parameter:
    positional = [parameter_to_python(parameter) for parameter in positional_parameters]
    named = {name: parameter_to_python(parameter) for name, parameter in named_parameters.items()}
    result = target(*positional, **named)
    return _result_to_parameter(result, reference)

def dispatch_message(message: Any) -> Any:
{{HANDLERS}}
    raise TypeError(f"Unsupported message: {type(message)!r}")
