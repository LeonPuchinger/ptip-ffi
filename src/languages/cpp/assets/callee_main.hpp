#pragma once

#include <array>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <stdexcept>
#include <string>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

#include "bridge.hpp"
#include "socket.hpp"

namespace ptip_ffi {

inline std::string resolve_library_path() {
    const char* value = std::getenv("FFI_LIBRARY_INVOKE");
    return value == nullptr ? std::string() : std::string(value);
}

class UnixDomainListener {
public:
    explicit UnixDomainListener(const std::string& socket_path) : socket_path_(socket_path) {
        socket_fd_ = socket(AF_UNIX, SOCK_STREAM, 0);
        if (socket_fd_ < 0) {
            throw std::runtime_error("failed to create unix listener");
        }

        sockaddr_un address{};
        std::memset(&address, 0, sizeof(address));
        address.sun_family = AF_UNIX;
        std::strncpy(address.sun_path, socket_path_.c_str(), sizeof(address.sun_path) - 1);

        unlink(socket_path_.c_str());
        if (bind(socket_fd_, reinterpret_cast<sockaddr*>(&address), sizeof(address)) < 0) {
            ::close(socket_fd_);
            socket_fd_ = -1;
            throw std::runtime_error("failed to bind unix listener");
        }
        if (listen(socket_fd_, 5) < 0) {
            ::close(socket_fd_);
            socket_fd_ = -1;
            throw std::runtime_error("failed to listen for unix connections");
        }
    }

    ~UnixDomainListener() {
        if (socket_fd_ >= 0) {
            ::close(socket_fd_);
            socket_fd_ = -1;
        }
        unlink(socket_path_.c_str());
    }

    int accept_connection() {
        int client_fd = ::accept(socket_fd_, nullptr, nullptr);
        if (client_fd < 0) {
            throw std::runtime_error("failed to accept client connection");
        }
        return client_fd;
    }

private:
    std::string socket_path_;
    int socket_fd_ = -1;
};

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
        SynchronousStream* stream = nullptr;
        class ClientStream : public SynchronousStream {
        public:
            explicit ClientStream(int fd) : fd_(fd) {}
            ~ClientStream() override {
                if (fd_ >= 0) {
                    ::close(fd_);
                    fd_ = -1;
                }
            }
            std::size_t read(char* buffer, std::size_t length) override {
                const ssize_t bytes_read = ::recv(fd_, buffer, length, 0);
                if (bytes_read <= 0) {
                    return 0;
                }
                return static_cast<std::size_t>(bytes_read);
            }
            std::size_t write(const char* buffer, std::size_t length) override {
                const ssize_t bytes_written = ::send(fd_, buffer, length, 0);
                if (bytes_written < 0) {
                    return 0;
                }
                return static_cast<std::size_t>(bytes_written);
            }
            void close() override {
                if (fd_ >= 0) {
                    ::close(fd_);
                    fd_ = -1;
                }
            }
        private:
            int fd_;
        } client_stream(client_fd);
        MessageSocket socket(client_stream);
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
