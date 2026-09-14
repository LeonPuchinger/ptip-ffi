#include "hashmap.grpc.pb.h"
#include <grpcpp/grpcpp.h>
#include <chrono>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <random>
#include <sstream>
#include <string>

int main(int argc, char** argv) {
    if (argc < 3) return EXIT_FAILURE;
    const int operations = std::stoi(argv[1]);
    const std::filesystem::path results_file = argv[2];
    const std::string address = "127.0.0.1:" + std::string(getenv("FFI_GRPC_PORT") ? getenv("FFI_GRPC_PORT") : "50051");
    auto channel = grpc::CreateChannel(address, grpc::InsecureChannelCredentials());
    auto stub = hashmap::HashMapService::NewStub(channel);
    grpc::ClientContext context;
    hashmap::Empty empty;
    hashmap::Handle handle;
    if (!stub->Create(&context, empty, &handle).ok()) return EXIT_FAILURE;
    const std::string alphabet = "abc";
    constexpr int max_value = 1000;
    std::mt19937 generator(std::random_device{}());
    std::uniform_int_distribution<int> length(1, 2), character(0, 2), value(0, max_value - 1);
    std::bernoulli_distribution write(0.5);
    const auto start = std::chrono::steady_clock::now();
    for (int i = 0; i < operations; ++i) {
        std::string key;
        const int key_length = length(generator);
        for (int j = 0; j < key_length; ++j) key += alphabet[character(generator)];
        hashmap::Key key_request;
        key_request.set_handle(handle.id()); key_request.set_key(key);
        if (write(generator)) {
            hashmap::Entry entry;
            entry.set_handle(handle.id()); entry.set_key(key); entry.set_value(value(generator));
            grpc::ClientContext call_context; hashmap::Empty response;
            if (!stub->Set(&call_context, entry, &response).ok()) return EXIT_FAILURE;
        } else {
            grpc::ClientContext has_context; hashmap::BoolValue exists;
            if (!stub->Has(&has_context, key_request, &exists).ok()) return EXIT_FAILURE;
            if (exists.value()) { grpc::ClientContext get_context; hashmap::Value response; if (!stub->Get(&get_context, key_request, &response).ok()) return EXIT_FAILURE; }
        }
    }
    const double elapsed = std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - start).count();
    const auto now = std::chrono::system_clock::now();
    const auto timestamp = std::chrono::system_clock::to_time_t(now);
    std::tm local = *std::localtime(&timestamp); char offset[16]{}; std::strftime(offset, sizeof(offset), "%z", &local);
    const auto milliseconds = std::chrono::duration_cast<std::chrono::milliseconds>(now.time_since_epoch()).count();
    std::ostringstream time; time << std::put_time(&local, "%Y-%m-%dT%H:%M:%S") << '.' << std::setfill('0') << std::setw(3) << milliseconds % 1000 << offset;
    std::filesystem::create_directories(results_file.parent_path());
    if (!std::filesystem::exists(results_file)) { std::ofstream header(results_file); header << "timestamp,operations,key_alphabet,max_insert_value,elapsed_ms\n"; }
    std::ofstream output(results_file, std::ios::app); output << time.str() << ',' << operations << ',' << alphabet << ',' << max_value << ',' << std::fixed << std::setprecision(3) << elapsed << '\n';
}
