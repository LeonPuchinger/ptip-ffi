from __future__ import annotations

import unittest

from ffi.bridge import CallMessage, FunctionTarget, Parameter, parse_message


class BridgeTests(unittest.TestCase):
    def test_call_message_roundtrip(self) -> None:
        message = CallMessage(
            module_path="",
            callee=FunctionTarget(name="trim_whitespace"),
            return_sink="352b6376-fff5-4dfa-8337-c85f175c349d",
            positional_parameters=[Parameter(kind="string", value="hello")],
        )

        serialized = message.serialize()
        parsed = parse_message(serialized)

        self.assertIsInstance(parsed, CallMessage)
        self.assertEqual(parsed.return_sink, message.return_sink)
        self.assertEqual(parsed.callee.name, message.callee.name)
        self.assertEqual(parsed.positional_parameters[0].value, "hello")


if __name__ == "__main__":
    unittest.main()
