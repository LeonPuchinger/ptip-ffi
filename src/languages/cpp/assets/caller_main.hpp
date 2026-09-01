#pragma once

#include <array>
#include <cstdio>
#include <cstdlib>
#include <memory>
#include <stdexcept>
#include <string>

#include "ffi/bridge.hpp"
#include "ffi/socket.hpp"

namespace ptip_ffi {

inline std::string resolve_library_invoke() {
    const char* value = std::getenv("FFI_LIBRARY_INVOKE");
    return value == nullptr ? std::string() : std::string(value);
}

inline std::string read_socket_path_from_command(const std::string& command) {
    std::array<char, 4096> buffer{};
    std::string output;

    FILE* pipe = popen(command.c_str(), "r");
    if (pipe == nullptr) {
        throw std::runtime_error("failed to start library process");
    }

    while (fgets(buffer.data(), static_cast<int>(buffer.size()), pipe) != nullptr) {
        output += buffer.data();
        if (output.find('\n') != std::string::npos) {
            break;
        }
    }

    // The library process is intentionally long-lived and stays alive for the lifetime of the caller.
    // We only need the first socket-path line to establish the bridge, and we must not block by
    // waiting for the child to exit here.
    std::string last_line;
    std::string current;
    std::stringstream stream(output);
    while (std::getline(stream, current)) {
        if (!current.empty()) {
            last_line = current;
        }
    }

    if (last_line.empty()) {
        throw std::runtime_error("library process did not print a socket path");
    }
    return last_line;
}

inline Bridge& establish_bridge() {
    static std::unique_ptr<Bridge> bridge;
    if (bridge != nullptr) {
        return *bridge;
    }

    const std::string invoke = resolve_library_invoke();
    if (invoke.empty()) {
        throw std::runtime_error("FFI_LIBRARY_INVOKE is not set");
    }

    const std::string socket_path = read_socket_path_from_command(invoke);
    bridge = std::make_unique<Bridge>(std::make_unique<UnixDomainStream>(socket_path));
    return *bridge;
}

inline Bridge& establishBridge() {
    return establish_bridge();
}

}
