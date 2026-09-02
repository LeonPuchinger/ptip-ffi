#pragma once

#include <any>
#include <string>
#include <unordered_map>
#include <vector>

#include "bridge.hpp"
#include "library/index.hpp"
#include "socket.hpp"

namespace ptip_ffi_generated {

inline std::unordered_map<std::string, std::any>& instance_registry() {
    static std::unordered_map<std::string, std::any> registry;
    return registry;
}

inline std::string respond_with_value(const std::string& sink, const ptip_ffi::Parameter& value) {
    return "S\n" + sink + "\n" + ptip_ffi::encode_parameter_line(value);
}

inline std::string respond_with_reference(const std::string& sink, const std::string& reference) {
    return "S\n" + sink + "\n" + ptip_ffi::encode_parameter_line(ptip_ffi::Parameter{ptip_ffi::ParameterKind::Reference, reference});
}

inline std::string decode_target_name(const std::string& encoded_name) {
    return ptip_ffi::decode_base64_no_pad_utf8(encoded_name);
}

template <typename T>
inline T decode_reference(const std::string& reference) {
    const auto& entry = instance_registry().at(reference);
    const auto* value = std::any_cast<T>(&entry);
    if (value == nullptr) {
        throw std::runtime_error("reference has an unexpected C++ type");
    }
    return *value;
}

template <typename T>
inline T* decode_reference_pointer(const std::string& reference) {
    auto& entry = instance_registry().at(reference);
    auto* value = std::any_cast<T>(&entry);
    if (value == nullptr) {
        throw std::runtime_error("reference has an unexpected C++ type");
    }
    return value;
}

inline std::string dispatch_message(const std::string& message) {
    if (message.empty()) {
        return "";
    }

    const auto lines = ptip_ffi::split_message_lines(message);
    if (lines.empty()) {
        return "";
    }

    const std::string kind = lines[0];
{{DISPATCH_CASES}}

    if (kind == "D") {
        if (lines.size() >= 2) {
            const std::string dropped_reference = lines[1];
            if (instance_registry().find(dropped_reference) != instance_registry().end()) {
                instance_registry().erase(dropped_reference);
            }
        }
        return "";
    }

    return "";
}

}
