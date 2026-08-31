#pragma once

#include <string>
#include <vector>

#include "bridge.hpp"
#include "socket.hpp"

namespace ptip_ffi_generated {

inline std::string dispatch_message(const std::string& message) {
    if (message.empty()) {
        return "";
    }

    const std::string kind = message.substr(0, message.find('\n'));
    if (kind == "C") {
        return "C";
    }
    if (kind == "M") {
        return "M";
    }
    if (kind == "R") {
        return "R";
    }
    if (kind == "U") {
        return "U";
    }
    if (kind == "S") {
        return "S";
    }
    if (kind == "A") {
        return "A";
    }
    if (kind == "E") {
        return "E";
    }
    if (kind == "D") {
        return "D";
    }
    return "";
}

{{DECLARATIONS}}

}  // namespace ptip_ffi_generated
