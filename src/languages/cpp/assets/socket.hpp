#pragma once

#include <array>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <stdexcept>
#include <string>
#include <sys/socket.h>
#include <sys/un.h>
#include <thread>
#include <unistd.h>
#include <vector>

namespace ptip_ffi {

class SynchronousStream {
public:
    virtual ~SynchronousStream() = default;
    virtual std::size_t read(char* buffer, std::size_t length) = 0;
    virtual std::size_t write(const char* buffer, std::size_t length) = 0;
    virtual void close() = 0;
};

class UnixDomainStream : public SynchronousStream {
public:
    explicit UnixDomainStream(int fd) : socket_fd_(fd) {}

    explicit UnixDomainStream(const std::string& socket_path) : socket_path_(socket_path) {
        constexpr int attempts = 50;
        constexpr auto delay = std::chrono::milliseconds(20);

        for (int attempt = 0; attempt < attempts; ++attempt) {
            socket_fd_ = socket(AF_UNIX, SOCK_STREAM, 0);
            if (socket_fd_ < 0) {
                throw std::runtime_error("failed to create unix socket");
            }

            sockaddr_un address{};
            std::memset(&address, 0, sizeof(address));
            address.sun_family = AF_UNIX;
            std::strncpy(address.sun_path, socket_path_.c_str(), sizeof(address.sun_path) - 1);

            if (connect(socket_fd_, reinterpret_cast<sockaddr*>(&address), sizeof(address)) == 0) {
                return;
            }

            ::close(socket_fd_);
            socket_fd_ = -1;

            if (attempt + 1 < attempts) {
                std::this_thread::sleep_for(delay);
            }
        }

        throw std::runtime_error("failed to connect to library socket");
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

class MessageSocket {
public:
    explicit MessageSocket(SynchronousStream& stream)
        : stream_(stream) {}

    void send(const std::string& data) {
        const std::string header = std::to_string(data.size()) + ":";
        stream_.write(header.c_str(), header.size());
        stream_.write(data.c_str(), data.size());
        stream_.write(",", 1);
    }

    std::string receive() {
        while (true) {
            const std::string raw = receive_raw();
            if (!raw.empty()) {
                return raw;
            }
            char buffer[4096];
            const std::size_t read_count = stream_.read(buffer, sizeof(buffer));
            if (read_count == 0) {
                return {};
            }
            buffer_.append(buffer, read_count);
        }
    }

    void close() {
        stream_.close();
    }

private:
    SynchronousStream& stream_;
    std::string buffer_;

    std::string receive_raw() {
        const std::size_t colon = buffer_.find(':');
        if (colon == std::string::npos) {
            return {};
        }

        const std::size_t length = std::stoull(buffer_.substr(0, colon));
        const std::size_t total = colon + 1 + length + 1;
        if (buffer_.size() < total) {
            return {};
        }

        const std::string message = buffer_.substr(colon + 1, length);
        if (buffer_[colon + 1 + length] != ',') {
            throw std::runtime_error("invalid netstring frame");
        }
        buffer_.erase(0, total);
        return message;
    }
};

}  // namespace ptip_ffi
