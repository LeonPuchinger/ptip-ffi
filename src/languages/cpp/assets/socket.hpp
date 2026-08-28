#pragma once

#include <cstddef>
#include <cstdint>
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
    explicit MessageSocket(SynchronousStream& stream);
    void send(const std::string& data);
    std::string receive();
    void close();

private:
    SynchronousStream& stream_;
    std::string buffer_;
    std::string receive_raw();
};

}  // namespace ptip_ffi
