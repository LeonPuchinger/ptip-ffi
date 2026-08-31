#pragma once

#include <algorithm>
#include <cstddef>
#include <deque>
#include <memory>
#include <stdexcept>
#include <string>
#include <utility>

#include "../../socket.hpp"

namespace ptip_ffi::tests {

struct SharedMemoryState {
    std::deque<std::string> queue;
    std::size_t max_read_size = 0;
    bool closed = false;
    bool remote_closed = false;
};

class MemoryDuplexStream final : public SynchronousStream {
public:
    MemoryDuplexStream() = default;

    static std::pair<MemoryDuplexStream, MemoryDuplexStream> pair() {
        auto left_state = std::make_shared<SharedMemoryState>();
        auto right_state = std::make_shared<SharedMemoryState>();

        MemoryDuplexStream left;
        MemoryDuplexStream right;
        left.state_ = left_state;
        right.state_ = right_state;
        left.peer_ = right_state;
        right.peer_ = left_state;
        return {std::move(left), std::move(right)};
    }

    void set_read_limit(std::size_t limit) {
        if (state_) {
            state_->max_read_size = limit;
        }
    }

    std::size_t read(char* buffer, std::size_t length) override {
        if (!state_ || state_->queue.empty()) {
            return 0;
        }

        const std::string& head = state_->queue.front();
        const std::size_t limit = state_->max_read_size == 0 ? length : std::min(length, state_->max_read_size);
        const std::size_t actual = std::min(limit, head.size());

        std::copy_n(head.begin(), actual, buffer);
        if (actual == head.size()) {
            state_->queue.pop_front();
        } else {
            state_->queue.front() = head.substr(actual);
        }

        return actual;
    }

    std::size_t write(const char* buffer, std::size_t length) override {
        if (!peer_) {
            throw std::runtime_error("Unpaired MemoryDuplexStream");
        }
        if (state_ && state_->closed) {
            throw std::runtime_error("Stream is closed");
        }
        peer_->queue.emplace_back(buffer, buffer + length);
        return length;
    }

    void close() override {
        if (!state_) {
            return;
        }
        state_->closed = true;
        if (peer_) {
            peer_->remote_closed = true;
        }
    }

private:
    std::shared_ptr<SharedMemoryState> state_;
    std::shared_ptr<SharedMemoryState> peer_;
};

}  // namespace ptip_ffi::tests
