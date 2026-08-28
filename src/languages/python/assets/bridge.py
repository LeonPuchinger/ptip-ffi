from __future__ import annotations

import base64
from dataclasses import dataclass, field
from typing import Any

from .socket import MessageSocket


def encode_base64_no_pad_utf8(text: str) -> str:
    return base64.b64encode(text.encode("utf-8")).decode("ascii").rstrip("=")


def decode_base64_no_pad_utf8(text: str) -> str:
    padding = "=" * (-len(text) % 4)
    return base64.b64decode((text + padding).encode("ascii")).decode("utf-8")


@dataclass(frozen=True)
class Parameter:
    kind: str
    value: Any


@dataclass(frozen=True)
class FunctionTarget:
    name: str


@dataclass(frozen=True)
class StaticMethodTarget:
    type_name: str
    method_name: str


CallTarget = FunctionTarget | StaticMethodTarget


@dataclass(frozen=True)
class CallMessage:
    module_path: str
    callee: CallTarget
    return_sink: str
    positional_parameters: list[Parameter] = field(default_factory=list)
    named_parameters: dict[str, Parameter] = field(default_factory=dict)

    def serialize(self) -> str:
        return "\n".join(
            [
                "C",
                serialize_invocation_path(self.module_path, self.callee),
                self.return_sink,
                *[encode_parameter_line(parameter) for parameter in self.positional_parameters],
                *[encode_parameter_line(parameter, name) for name, parameter in self.named_parameters.items()],
            ]
        )


@dataclass(frozen=True)
class MethodMessage:
    called_reference: str
    method_name: str
    return_sink: str
    positional_parameters: list[Parameter] = field(default_factory=list)
    named_parameters: dict[str, Parameter] = field(default_factory=dict)

    def serialize(self) -> str:
        return "\n".join(
            [
                "M",
                self.called_reference,
                encode_base64_no_pad_utf8(self.method_name),
                self.return_sink,
                *[encode_parameter_line(parameter) for parameter in self.positional_parameters],
                *[encode_parameter_line(parameter, name) for name, parameter in self.named_parameters.items()],
            ]
        )


@dataclass(frozen=True)
class RequestMessage:
    parent: str
    accessor: str
    valueSink: str

    def serialize(self) -> str:
        return "\n".join(["R", self.parent, encode_base64_no_pad_utf8(self.accessor), self.valueSink])


@dataclass(frozen=True)
class UpdateMessage:
    parent: str
    accessor: str
    acknowledgeSink: str
    value: Parameter

    def serialize(self) -> str:
        return "\n".join(
            ["U", self.parent, encode_base64_no_pad_utf8(self.accessor), self.acknowledgeSink, encode_parameter_line(self.value)]
        )


@dataclass(frozen=True)
class SendMessage:
    reference: str
    value: Parameter

    def serialize(self) -> str:
        return "\n".join(["S", self.reference, encode_parameter_line(self.value)])


@dataclass(frozen=True)
class AcknowledgeMessage:
    reference: str

    def serialize(self) -> str:
        return "\n".join(["A", self.reference])


@dataclass(frozen=True)
class ErrorMessage:
    reference: str
    error: Parameter

    def serialize(self) -> str:
        return "\n".join(["E", self.reference, encode_parameter_line(self.error)])


@dataclass(frozen=True)
class DropMessage:
    reference: str

    def serialize(self) -> str:
        return "\n".join(["D", self.reference])


Message = CallMessage | MethodMessage | RequestMessage | UpdateMessage | SendMessage | AcknowledgeMessage | ErrorMessage | DropMessage


class Bridge:
    def __init__(self, socket: MessageSocket):
        self.socket = socket

    def send(self, message: Message) -> None:
        self.socket.send_text(message.serialize())

    def next_message(self) -> Message | None:
        text = self.socket.receive_text()
        if text is None:
            return None
        return parse_message(text)

    def close(self) -> None:
        self.socket.close()


def serialize_invocation_path(module_path: str, callee: CallTarget) -> str:
    if isinstance(callee, FunctionTarget):
        if module_path:
            return f"{module_path}.{callee.name}"
        return callee.name
    if isinstance(callee, StaticMethodTarget):
        if module_path:
            return f"{module_path}:{callee.type_name}#{callee.method_name}"
        return f"{callee.type_name}#{callee.method_name}"
    raise TypeError(f"Unsupported callee target: {type(callee)!r}")


def encode_parameter_line(parameter: Parameter, name: str | None = None) -> str:
    value = serialize_parameter(parameter)
    if name is None:
        return value
    return f"{value} {encode_base64_no_pad_utf8(name)}"


def serialize_parameter(parameter: Parameter) -> str:
    if parameter.kind == "integer":
        return f"i{int(parameter.value):x}"
    if parameter.kind == "float":
        return f"f{parameter.value}"
    if parameter.kind == "boolean":
        return f"b{1 if parameter.value else 0}"
    if parameter.kind == "string":
        return f"s{encode_base64_no_pad_utf8(str(parameter.value))}"
    if parameter.kind == "reference":
        return f"r{parameter.value}"
    raise TypeError(f"Unsupported parameter kind: {parameter.kind!r}")


def parse_parameter_line(text: str) -> tuple[Parameter, str | None]:
    value_text, *rest = text.split(" ", 1)
    parameter = parse_parameter(value_text)
    if rest:
        return parameter, decode_base64_no_pad_utf8(rest[0])
    return parameter, None


def parse_parameter(text: str) -> Parameter:
    if not text:
        raise ValueError("Empty parameter")
    kind = text[0]
    value = text[1:]
    if kind == "i":
        return Parameter(kind="integer", value=int(value, 16))
    if kind == "f":
        return Parameter(kind="float", value=float(value))
    if kind == "b":
        return Parameter(kind="boolean", value=value == "1")
    if kind == "s":
        return Parameter(kind="string", value=decode_base64_no_pad_utf8(value))
    if kind == "r":
        return Parameter(kind="reference", value=value)
    raise ValueError(f"Unsupported parameter descriptor: {kind!r}")


def python_to_parameter(value: Any) -> Parameter:
    if isinstance(value, Parameter):
        return value
    if isinstance(value, bool):
        return Parameter(kind="boolean", value=value)
    if isinstance(value, int) and not isinstance(value, bool):
        return Parameter(kind="integer", value=value)
    if isinstance(value, float):
        return Parameter(kind="float", value=value)
    if isinstance(value, str):
        return Parameter(kind="string", value=value)
    if hasattr(value, "uuid"):
        return Parameter(kind="reference", value=str(getattr(value, "uuid")))
    return Parameter(kind="string", value=str(value))


def parameter_to_python(parameter: Parameter) -> Any:
    if parameter.kind in {"integer", "float", "boolean", "string"}:
        return parameter.value
    if parameter.kind == "reference":
        return parameter.value
    raise TypeError(f"Unsupported parameter kind: {parameter.kind!r}")


def parse_message(text: str) -> Message:
    lines = text.split("\n")
    if not lines:
        raise ValueError("Empty message")

    kind = lines[0]
    if kind == "C":
        module_path, callee = parse_invocation_path(lines[1])
        positional: list[Parameter] = []
        named: dict[str, Parameter] = {}
        for line in lines[3:]:
            parameter, name = parse_parameter_line(line)
            if name is None:
                positional.append(parameter)
            else:
                named[name] = parameter
        return CallMessage(
            module_path=module_path,
            callee=callee,
            return_sink=lines[2],
            positional_parameters=positional,
            named_parameters=named,
        )
    if kind == "M":
        positional = []
        named = {}
        for line in lines[4:]:
            parameter, name = parse_parameter_line(line)
            if name is None:
                positional.append(parameter)
            else:
                named[name] = parameter
        return MethodMessage(
            called_reference=lines[1],
            method_name=decode_base64_no_pad_utf8(lines[2]),
            return_sink=lines[3],
            positional_parameters=positional,
            named_parameters=named,
        )
    if kind == "R":
        return RequestMessage(parent=lines[1], accessor=decode_base64_no_pad_utf8(lines[2]), valueSink=lines[3])
    if kind == "U":
        return UpdateMessage(parent=lines[1], accessor=decode_base64_no_pad_utf8(lines[2]), acknowledgeSink=lines[3], value=parse_parameter(lines[4]))
    if kind == "S":
        return SendMessage(reference=lines[1], value=parse_parameter(lines[2]))
    if kind == "A":
        return AcknowledgeMessage(reference=lines[1])
    if kind == "E":
        return ErrorMessage(reference=lines[1], error=parse_parameter(lines[2]))
    if kind == "D":
        return DropMessage(reference=lines[1])
    raise ValueError(f"Unsupported message kind: {kind!r}")


def parse_invocation_path(text: str) -> tuple[str, CallTarget]:
    if ":" in text and "#" in text:
        module_path, rest = text.split(":", 1)
        type_name, method_name = rest.split("#", 1)
        return module_path, StaticMethodTarget(type_name=type_name, method_name=method_name)
    if "." in text:
        module_path, function_name = text.rsplit(".", 1)
        return module_path, FunctionTarget(name=function_name)
    if "#" in text:
        type_name, method_name = text.split("#", 1)
        return "", StaticMethodTarget(type_name=type_name, method_name=method_name)
    return "", FunctionTarget(name=text)
