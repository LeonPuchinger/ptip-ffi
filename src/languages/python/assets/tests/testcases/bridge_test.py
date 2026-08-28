from __future__ import annotations

import unittest

from ffi.bridge import (
    AcknowledgeMessage,
    CallMessage,
    DropMessage,
    ErrorMessage,
    FunctionTarget,
    MethodMessage,
    Parameter,
    RequestMessage,
    SendMessage,
    StaticMethodTarget,
    UpdateMessage,
    parse_invocation_path,
    parse_message,
)


class BridgeTests(unittest.TestCase):
    def test_parse_invocation_path_supports_all_targets(self) -> None:
        module_path, callee = parse_invocation_path("trim_whitespace")
        self.assertEqual(module_path, "")
        self.assertEqual(callee, FunctionTarget(name="trim_whitespace"))

        module_path, callee = parse_invocation_path("pkg/util.trim_whitespace")
        self.assertEqual(module_path, "pkg/util")
        self.assertEqual(callee, FunctionTarget(name="trim_whitespace"))

        module_path, callee = parse_invocation_path("Widget#create")
        self.assertEqual(module_path, "")
        self.assertEqual(callee, StaticMethodTarget(type_name="Widget", method_name="create"))

        module_path, callee = parse_invocation_path("pkg/util:Widget#create")
        self.assertEqual(module_path, "pkg/util")
        self.assertEqual(callee, StaticMethodTarget(type_name="Widget", method_name="create"))

    def test_call_message_roundtrip_with_positional_and_named_parameters(self) -> None:
        message = CallMessage(
            module_path="foo/bar",
            callee=FunctionTarget(name="sum"),
            return_sink="352b6376-fff5-4dfa-8337-c85f175c349d",
            positional_parameters=[
                Parameter(kind="integer", value=-1),
                Parameter(kind="float", value=3.5),
                Parameter(kind="boolean", value=True),
                Parameter(kind="string", value="Hello ü"),
                Parameter(kind="reference", value="ref-uuid"),
            ],
            named_parameters={
                "x": Parameter(kind="integer", value=255),
                "title": Parameter(kind="string", value="Hello ü"),
            },
        )

        serialized = message.serialize()
        parsed = parse_message(serialized)

        self.assertIsInstance(parsed, CallMessage)
        self.assertEqual(parsed.module_path, "foo/bar")
        self.assertEqual(parsed.return_sink, message.return_sink)
        self.assertEqual(parsed.callee.name, message.callee.name)
        self.assertEqual(parsed.positional_parameters, message.positional_parameters)
        self.assertEqual(parsed.named_parameters, message.named_parameters)

    def test_static_method_call_roundtrip(self) -> None:
        message = CallMessage(
            module_path="pkg/util",
            callee=StaticMethodTarget(type_name="Widget", method_name="create"),
            return_sink="sink",
        )

        parsed = parse_message(message.serialize())

        self.assertIsInstance(parsed, CallMessage)
        self.assertEqual(parsed.module_path, "pkg/util")
        self.assertEqual(parsed.callee, StaticMethodTarget(type_name="Widget", method_name="create"))
        self.assertEqual(parsed.return_sink, "sink")

    def test_roundtrips_remaining_message_kinds(self) -> None:
        messages = [
            MethodMessage(
                called_reference="ref-1",
                method_name="doThing",
                return_sink="sink-1",
                positional_parameters=[Parameter(kind="integer", value=42)],
                named_parameters={"label": Parameter(kind="string", value="ready")},
            ),
            RequestMessage(parent="parent-1", accessor="field.name", valueSink="value-1"),
            UpdateMessage(
                parent="parent-1",
                accessor="field.name",
                acknowledgeSink="ack-1",
                value=Parameter(kind="boolean", value=False),
            ),
            SendMessage(reference="ref-2", value=Parameter(kind="string", value="payload")),
            AcknowledgeMessage(reference="ref-2"),
            ErrorMessage(reference="ref-2", error=Parameter(kind="string", value="boom")),
            DropMessage(reference="ref-2"),
        ]

        for message in messages:
            parsed = parse_message(message.serialize())
            self.assertEqual(parsed, message)

    def test_parse_message_rejects_unknown_kind(self) -> None:
        with self.assertRaisesRegex(ValueError, "Unsupported message kind"):
            parse_message("Z")

    def test_parameter_roundtrip_helpers(self) -> None:
        from ffi.bridge import parameter_to_python, python_to_parameter

        self.assertEqual(python_to_parameter(True), Parameter(kind="boolean", value=True))
        self.assertEqual(python_to_parameter(12), Parameter(kind="integer", value=12))
        self.assertEqual(python_to_parameter(3.25), Parameter(kind="float", value=3.25))
        self.assertEqual(python_to_parameter("hello"), Parameter(kind="string", value="hello"))
        self.assertEqual(parameter_to_python(Parameter(kind="reference", value="ref-1")), "ref-1")


if __name__ == "__main__":
    unittest.main()
