#include <cassert>
#include <string>

#include "stream.hpp"

int main() {
    using ptip_ffi::MessageSocket;

    {
        auto [left, right] = ptip_ffi::tests::MemoryDuplexStream::pair();
        MessageSocket sender(left);
        MessageSocket receiver(right);

        sender.send("hello");
        assert(receiver.receive() == "hello");
    }

    {
        auto [left, right] = ptip_ffi::tests::MemoryDuplexStream::pair();
        right.set_read_limit(1);
        MessageSocket sender(left);
        MessageSocket receiver(right);

        sender.send("fragmented message");
        assert(receiver.receive() == "fragmented message");
    }

    {
        auto [left, right] = ptip_ffi::tests::MemoryDuplexStream::pair();
        left.write("5:hello,5:world,", 16);
        MessageSocket receiver(right);
        assert(receiver.receive() == "hello");
        assert(receiver.receive() == "world");
    }

    {
        auto [left, right] = ptip_ffi::tests::MemoryDuplexStream::pair();
        left.write("x:abc,", 6);
        MessageSocket receiver(right);
        bool threw = false;
        try {
            receiver.receive();
        } catch (const std::exception&) {
            threw = true;
        }
        assert(threw);
    }

    return 0;
}
