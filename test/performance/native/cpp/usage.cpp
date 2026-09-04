#include "../../input/cpp/index.hpp"

#include <chrono>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <random>
#include <sstream>
#include <string>

int main(int argc, char **argv) {
    int operations = -1;
    if (argc > 1) {
        try {
            operations = std::stoi(argv[1]);
        } catch (const std::exception &) {
            operations = -1;
        }
    }

    if (operations <= 0) {
        std::cerr << "Please provide a positive integer for the number of operations.\n";
        return EXIT_FAILURE;
    }

    const std::string key_alphabet = "abc";
    constexpr int max_insert_value = 1000;
    std::random_device random_device;
    std::mt19937 generator(random_device());
    std::uniform_int_distribution<int> key_length_distribution(1, 2);
    std::uniform_int_distribution<int> alphabet_distribution(0, key_alphabet.size() - 1);
    std::uniform_int_distribution<int> value_distribution(0, max_insert_value - 1);
    std::bernoulli_distribution write_distribution(0.5);

    const auto start = std::chrono::steady_clock::now();

    HashMap<std::string, int> map;

    for (int i = 0; i < operations; ++i) {
        const int key_length = key_length_distribution(generator);
        std::string key;
        key.reserve(key_length);
        for (int j = 0; j < key_length; ++j) {
            key += key_alphabet[alphabet_distribution(generator)];
        }
        const int value = value_distribution(generator);
        if (write_distribution(generator)) {
            map.set(key, value);
        } else if (map.has(key)) {
            map.get(key);
        }
    }

    const auto elapsed = std::chrono::duration<double, std::milli>(
        std::chrono::steady_clock::now() - start
    ).count();

    const auto now = std::chrono::system_clock::now();
    const auto milliseconds = std::chrono::duration_cast<std::chrono::milliseconds>(
        now.time_since_epoch()
    ).count();
    const auto seconds = std::chrono::duration_cast<std::chrono::seconds>(
        now.time_since_epoch()
    );
    const auto fractional_milliseconds = milliseconds -
        std::chrono::duration_cast<std::chrono::milliseconds>(seconds).count();
    const std::time_t timestamp = std::chrono::system_clock::to_time_t(now);
    std::tm local_timestamp = *std::localtime(&timestamp);
    char timezone_offset[16]{};
    std::strftime(timezone_offset, sizeof(timezone_offset), "%z", &local_timestamp);
    std::ostringstream timestamp_stream;
    timestamp_stream << std::put_time(&local_timestamp, "%Y-%m-%dT%H:%M:%S")
                     << '.' << std::setfill('0') << std::setw(3) << fractional_milliseconds
                     << timezone_offset;

    const std::filesystem::path results_dir =
        std::filesystem::path(__FILE__).parent_path() / "../../results";
    const std::filesystem::path results_file = results_dir / "native_cpp.csv";
    std::filesystem::create_directories(results_dir);
    if (!std::filesystem::exists(results_file)) {
        std::ofstream header(results_file);
        header << "timestamp,operations,key_alphabet,max_insert_value,elapsed_ms\n";
    }

    std::ofstream output(results_file, std::ios::app);
    output << timestamp_stream.str() << ',' << operations << ',' << key_alphabet << ','
           << max_insert_value << ',' << std::fixed << std::setprecision(3) << elapsed << '\n';
}
