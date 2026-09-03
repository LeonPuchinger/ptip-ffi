#include <cmath>
#include <iostream>
#include <stdexcept>
#include <string>

#include "caller/index.hpp"

int main() {
    const std::string trimmed = ptip_ffi_generated::trim_whitespace("  hello from cpp  ");
    if (trimmed != "hello from cpp") {
        throw std::runtime_error("Unexpected trimmed value");
    }

    ptip_ffi_generated::Point point(3, 4);
    const double distance = point.distance_to_origin();
    if (std::abs(distance - 5.0) > 1e-9) {
        throw std::runtime_error("Unexpected distance");
    }

    const double distance2 = ptip_ffi_generated::takes_point(point);
    if (distance2 != distance) {
        throw std::runtime_error("Unexpected distance from takes_point");
    }

    ptip_ffi_generated::LinkedList<int> values;
    values.append(42);
    if (values.get(0) != 42) {
        throw std::runtime_error("Unexpected list value");
    }

    ptip_ffi_generated::HashMap<std::string, int> map;
    map.set("answer", 42);
    if (map.get("answer") != 42 || !map.has("answer") || !map.remove("answer")) {
        throw std::runtime_error("Unexpected map result");
    }

    std::cout << "CPP caller integration passed" << std::endl;
    return 0;
}
