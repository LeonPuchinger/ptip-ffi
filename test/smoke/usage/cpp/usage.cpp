#include <any>
#include <cmath>
#include <iostream>
#include <stdexcept>
#include <string>

#include "caller/index.hpp"

int main() {
    const std::any trimmed = ptip_ffi_generated::trim_whitespace(std::string("  hello from cpp  "));
    if (std::any_cast<std::string>(trimmed) != "hello from cpp") {
        throw std::runtime_error("Unexpected trimmed value");
    }

    ptip_ffi_generated::Point point(3, 4);
    const std::any distance = point.distance_to_origin();
    if (std::abs(std::any_cast<double>(distance) - 5.0) > 1e-9) {
        throw std::runtime_error("Unexpected distance");
    }

    ptip_ffi_generated::LinkedList values;
    values.append(42);
    if (std::any_cast<long long>(values.get(0)) != 42) {
        throw std::runtime_error("Unexpected list value");
    }

    ptip_ffi_generated::HashMap map;
    map.set("answer", 42);
    if (std::any_cast<long long>(map.get(std::string("answer"))) != 42
        || !std::any_cast<bool>(map.has(std::string("answer")))
        || !std::any_cast<bool>(map.remove(std::string("answer")))) {
        throw std::runtime_error("Unexpected map result");
    }

    std::cout << "CPP caller integration passed" << std::endl;
    return 0;
}
