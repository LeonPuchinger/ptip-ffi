#include <cassert>
#include <string>

#include "../bridge.hpp"

int main() {
    const ptip_ffi::Parameter integer_value{ptip_ffi::ParameterKind::Integer, "42"};
    assert(ptip_ffi::encode_parameter_line(integer_value) == "i2a");

    const ptip_ffi::Parameter string_value{ptip_ffi::ParameterKind::String, "Hello"};
    assert(ptip_ffi::encode_parameter_line(string_value) == "sSGVsbG8");

    const ptip_ffi::Parameter decoded = ptip_ffi::decode_parameter_line("sSGVsbG8");
    assert(decoded.kind == ptip_ffi::ParameterKind::String);
    assert(decoded.value == "Hello");

    assert(ptip_ffi::serialize_invocation_path("foo/bar", "sum") == "Zm9v/YmFy.c3Vt");
    assert(ptip_ffi::serialize_invocation_path("", "sum") == "c3Vt");

    const auto lines = ptip_ffi::split_message_lines("A\nB\nC");
    assert(lines.size() == 3);
    assert(lines[0] == "A");
    assert(lines[1] == "B");
    assert(lines[2] == "C");

    return 0;
}
