#pragma once

#include <array>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <stdexcept>
#include <string>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

#include "ffi/bridge.hpp"
#include "ffi/socket.hpp"

namespace ptip_ffi {

inline std::uint64_t next_uuid_counter() {
    static std::uint64_t counter = 0;
    return ++counter;
}

inline std::string generate_uuid() {
    return "uuid-" + std::to_string(next_uuid_counter());
}

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
    }

    const int status = pclose(pipe);
    if (status != 0) {
        throw std::runtime_error("library process exited with a non-zero status");
    }

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

class UnixDomainStream : public SynchronousStream {
public:
    explicit UnixDomainStream(const std::string& socket_path) : socket_path_(socket_path) {
        socket_fd_ = socket(AF_UNIX, SOCK_STREAM, 0);
        if (socket_fd_ < 0) {
            throw std::runtime_error("failed to create unix socket");
        }

        sockaddr_un address{};
        std::memset(&address, 0, sizeof(address));
        address.sun_family = AF_UNIX;
        std::strncpy(address.sun_path, socket_path_.c_str(), sizeof(address.sun_path) - 1);

        if (connect(socket_fd_, reinterpret_cast<sockaddr*>(&address), sizeof(address)) < 0) {
            ::close(socket_fd_);
            socket_fd_ = -1;
            throw std::runtime_error("failed to connect to library socket");
        }
    }

    ~UnixDomainStream() override {
        close();
    }

    std::size_t read(char* buffer, std::size_t length) override {
        const ssize_t bytes_read = ::recv(socket_fd_, buffer, length, 0);
        if (bytes_read <= 0) {
            return 0;
        }
        return static_cast<std::size_t>(bytes_read);
    }

    std::size_t write(const char* buffer, std::size_t length) override {
        const ssize_t bytes_written = ::send(socket_fd_, buffer, length, 0);
        if (bytes_written < 0) {
            return 0;
        }
        return static_cast<std::size_t>(bytes_written);
    }

    void close() override {
        if (socket_fd_ >= 0) {
            ::close(socket_fd_);
            socket_fd_ = -1;
        }
    }

private:
    std::string socket_path_;
    int socket_fd_ = -1;
};

inline Bridge establish_bridge() {
    static std::unique_ptr<Bridge> bridge;
    if (bridge != nullptr) {
        return *bridge;
    }

    const std::string invoke = resolve_library_invoke();
    if (invoke.empty()) {
        throw std::runtime_error("FFI_LIBRARY_INVOKE is not set");
    }

    const std::string socket_path = read_socket_path_from_command(invoke);
    auto stream = std::make_unique<UnixDomainStream>(socket_path);
    auto socket = std::make_unique<MessageSocket>(*stream);
    bridge = std::make_unique<Bridge>(*socket);
    return *bridge;
}

inline Bridge establishBridge() {
    return establish_bridge();
}

class ManagedReference {
public:
    ManagedReference(std::string uuid, Bridge* bridge) : uuid_(std::move(uuid)), bridge_(bridge) {}
    ~ManagedReference() {
        if (bridge_ != nullptr && !uuid_.empty()) {
            bridge_->send_message("D\n" + uuid_);
        }
    }

    ManagedReference(const ManagedReference&) = delete;
    ManagedReference& operator=(const ManagedReference&) = delete;

    ManagedReference(ManagedReference&& other) noexcept : uuid_(std::move(other.uuid_)), bridge_(other.bridge_) {
        other.bridge_ = nullptr;
        other.uuid_.clear();
    }

    ManagedReference& operator=(ManagedReference&& other) noexcept {
        if (this != &other) {
            if (bridge_ != nullptr && !uuid_.empty()) {
                bridge_->send_message("D\n" + uuid_);
            }
            uuid_ = std::move(other.uuid_);
            bridge_ = other.bridge_;
            other.bridge_ = nullptr;
            other.uuid_.clear();
        }
        return *this;
    }

    const std::string& uuid() const {
        return uuid_;
    }

private:
    std::string uuid_;
    Bridge* bridge_ = nullptr;
};

}  // namespace ptip_ffi
