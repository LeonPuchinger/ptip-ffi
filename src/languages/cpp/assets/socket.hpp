#pragma once

#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

namespace ptip_ffi {

class SynchronousStream {
public:
    virtual ~SynchronousStream() = default;
    virtual std::size_t read(char* buffer, std::size_t length) = 0;
    virtual std::size_t write(const char* buffer, std::size_t length) = 0;
    virtual void close() = 0;
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
