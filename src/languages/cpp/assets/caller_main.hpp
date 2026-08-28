#pragma once

#include <cstdlib>
#include <iostream>
#include <string>

namespace ptip_ffi {

inline std::string resolve_library_invoke() {
    const char* value = std::getenv("FFI_LIBRARY_INVOKE");
    return value == nullptr ? std::string() : std::string(value);
}

}  // namespace ptip_ffi
