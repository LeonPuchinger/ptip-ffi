#include <any>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <random>
#include <sstream>
#include <string>
#include <type_traits>

#include "caller/index.hpp"

namespace {

const std::string &as_string(const std::string &value) {
    return value;
}

std::string as_string(const std::any &value) {
    return std::any_cast<std::string>(value);
}

template <typename T>
bool as_bool(T value) {
    if constexpr (std::is_same_v<T, std::any>) {
        return std::any_cast<bool>(value);
    } else {
        return value;
    }
}

template <typename T>
long long as_integer(T value) {
    if constexpr (std::is_same_v<T, std::any>) {
        if (value.type() == typeid(long long)) {
            return std::any_cast<long long>(value);
        }
        return std::any_cast<int>(value);
    } else {
        return static_cast<long long>(value);
    }
}

} // namespace

int main(int argc, char **argv) {
    if (argc < 3) {
        std::cerr << "Usage: usage.cpp OPERATIONS RESULTS_FILE\n";
        return EXIT_FAILURE;
    }

    int operations;
    try {
        operations = std::stoi(argv[1]);
    } catch (const std::exception &) {
        operations = -1;
    }
    if (operations <= 0) {
        std::cerr << "Please provide a positive integer for the number of operations.\n";
        return EXIT_FAILURE;
    }

    const std::string results_file = argv[2];
    const std::string key_alphabet = "abc";
    constexpr int max_insert_value = 1000;
    std::random_device random_device;
    std::mt19937 generator(random_device());
    std::uniform_int_distribution<int> key_length_distribution(1, 2);
    std::uniform_int_distribution<int> alphabet_distribution(0, key_alphabet.size() - 1);
    std::uniform_int_distribution<int> value_distribution(0, max_insert_value - 1);
    std::bernoulli_distribution write_distribution(0.5);

    const auto start = std::chrono::steady_clock::now();
    ptip_ffi_generated::HashMap map;

    for (int i = 0; i < operations; ++i) {
        const int key_length = key_length_distribution(generator);
        std::string key;
        key.reserve(key_length);
        for (int j = 0; j < key_length; ++j) {
            key += key_alphabet[alphabet_distribution(generator)];
        }
        const long long value = value_distribution(generator);
        if (write_distribution(generator)) {
            map.set(key, value);
        } else if (as_bool(map.has(key))) {
            map.get(key);
        }
    }

    const auto elapsed = std::chrono::duration<double, std::milli>(
        std::chrono::steady_clock::now() - start
    ).count();
    const auto now = std::chrono::system_clock::now();
    const auto timestamp = std::chrono::system_clock::to_time_t(now);
        std::tm local_timestamp = *std::localtime(&timestamp);
        char timezone_offset[16]{};
        std::strftime(timezone_offset, sizeof(timezone_offset), "%z", &local_timestamp);
    const auto milliseconds = std::chrono::duration_cast<std::chrono::milliseconds>(now.time_since_epoch()).count();
    const auto fractional = milliseconds % 1000;
    std::ostringstream timestamp_stream;
        timestamp_stream << std::put_time(&local_timestamp, "%Y-%m-%dT%H:%M:%S")
                         << '.' << std::setfill('0') << std::setw(3) << fractional
                         << timezone_offset;

    const std::filesystem::path path(results_file);
    std::filesystem::create_directories(path.parent_path());
    if (!std::filesystem::exists(path)) {
        std::ofstream header(path);
        header << "timestamp,operations,key_alphabet,max_insert_value,elapsed_ms\n";
    }
    std::ofstream output(path, std::ios::app);
    output << timestamp_stream.str() << ',' << operations << ',' << key_alphabet << ','
           << max_insert_value << ',' << std::fixed << std::setprecision(3) << elapsed << '\n';
    return 0;
}
