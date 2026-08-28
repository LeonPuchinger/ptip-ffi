#include <cassert>
#include <iostream>
#include <string>

#include "../bridge.hpp"

int main() {
    ptip_ffi::Parameter value{ptip_ffi::ParameterKind::Integer, "42"};
    auto encoded = ptip_ffi::encode_parameter_line(value);
    assert(encoded == "i2a");
    assert(ptip_ffi::serialize_invocation_path("foo/bar", "sum") == "Zm9vL2Jhcg==.c3Vt");
    std::cout << "cpp bridge smoke ok\n";
    return 0;
}
