#pragma once

#include <chrono>
#include <cstdlib>
#include <iostream>
#include <stdexcept>
#include <string>
#include <unistd.h>

#include "bridge.hpp"
#include "dispatch.hpp"
#include "socket.hpp"

namespace ptip_ffi {

inline int run_library_server() {
    const auto now = std::chrono::steady_clock::now().time_since_epoch().count();
    const std::string socket_path = "/tmp/ptip_ffi_cpp_" + std::to_string(getpid()) + "_" + std::to_string(now) + ".sock";

    UnixDomainListener listener(socket_path);

    std::cout << socket_path << std::endl;
    std::cout.flush();

    while (true) {
        const int client_fd = listener.accept_connection();
        UnixDomainStream stream(client_fd);
        MessageSocket socket(stream);
        while (true) {
            const std::string payload = socket.receive();
            if (payload.empty()) {
                break;
            }
            const std::string response = ptip_ffi_generated::dispatch_message(payload);
            if (!response.empty()) {
                socket.send(response);
            }
        }
    }
    return 0;
}

}
