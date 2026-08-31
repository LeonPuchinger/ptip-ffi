#pragma once

#include <cstdlib>
#include <iostream>
#include <stdexcept>
#include <string>

#include "bridge.hpp"
#include "dispatch.hpp"
#include "socket.hpp"

namespace ptip_ffi {

inline std::string resolve_library_path() {
    const char* value = std::getenv("FFI_LIBRARY_INVOKE");
    return value == nullptr ? std::string() : std::string(value);
}

inline int run_library_server() {
    const std::string socket_path = resolve_library_path();
    if (socket_path.empty()) {
        throw std::runtime_error("FFI_LIBRARY_INVOKE is not set");
    }

    std::cout << socket_path << std::endl;
    std::cout.flush();

    UnixDomainListener listener(socket_path);
    while (true) {
        const int client_fd = listener.accept_connection();
        UnixDomainStream stream(client_fd);
        MessageSocket socket(stream);
        while (true) {
            const std::string payload = socket.receive();
            if (payload.empty()) {
                break;
            }
            (void)payload;
        }
    }
    return 0;
}

}  // namespace ptip_ffi
