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

inline std::string dispatch_message(const std::string& message) {
    if (message.empty()) {
        return "";
    }

    const auto lines = ptip_ffi::split_message_lines(message);
    if (lines.empty()) {
        return "";
    }

    const std::string kind = lines[0];
    if (kind == "C") {
        if (lines.size() < 3) {
            return "";
        }

        const std::string target = decode_target_name(lines[1]);
        const std::string return_sink = lines[2];
        const auto return_param = [&]() -> ptip_ffi::Parameter {
            if (target == "trim_whitespace") {
                if (lines.size() < 4) {
                    throw std::runtime_error("trim_whitespace requires a string argument");
                }
                const auto value = ptip_ffi::decode_parameter_line(lines[3]);
                const std::string value_string = ptip_ffi::decode_value<std::string>(value);
                return ptip_ffi::encode_value(trim_whitespace(value_string));
            }
            if (target == "Point") {
                if (lines.size() < 5) {
                    throw std::runtime_error("Point constructor requires two arguments");
                }
                const auto x_value = ptip_ffi::decode_parameter_line(lines[3]);
                const auto y_value = ptip_ffi::decode_parameter_line(lines[4]);
                if (x_value.kind == ptip_ffi::ParameterKind::Integer && y_value.kind == ptip_ffi::ParameterKind::Integer) {
                    const auto instance = Point<int>(ptip_ffi::decode_value<int>(x_value), ptip_ffi::decode_value<int>(y_value));
                    instance_registry().emplace(return_sink, instance);
                    return ptip_ffi::Parameter{ptip_ffi::ParameterKind::Reference, return_sink};
                }
                const auto instance = Point<double>(ptip_ffi::decode_value<double>(x_value), ptip_ffi::decode_value<double>(y_value));
                instance_registry().emplace(return_sink, instance);
                return ptip_ffi::Parameter{ptip_ffi::ParameterKind::Reference, return_sink};
            }
            if (target == "LinkedList") {
                instance_registry().emplace(return_sink, LinkedList<int>{});
                return ptip_ffi::Parameter{ptip_ffi::ParameterKind::Reference, return_sink};
            }
            if (target == "HashMap") {
                instance_registry().emplace(return_sink, HashMap<std::string, int>{});
                return ptip_ffi::Parameter{ptip_ffi::ParameterKind::Reference, return_sink};
            }
            if (target == "takes_point") {
                if (lines.size() < 4) {
                    throw std::runtime_error("takes_point requires a Point reference");
                }
                const auto value = ptip_ffi::decode_parameter_line(lines[3]);
                if (value.kind != ptip_ffi::ParameterKind::Reference) {
                    throw std::runtime_error("takes_point expects a reference");
                }
                const auto& entry = instance_registry().at(value.value);
                if (const auto* instance = std::any_cast<Point<int>>(&entry)) {
                    return ptip_ffi::encode_value(takes_point(*instance));
                }
                if (const auto* instance = std::any_cast<Point<double>>(&entry)) {
                    return ptip_ffi::encode_value(takes_point(*instance));
                }
                throw std::runtime_error("takes_point received an unsupported Point type");
            }
            throw std::runtime_error("unknown function target: " + target);
        }();
        return respond_with_value(return_sink, return_param);
    }

    if (kind == "M") {
        if (lines.size() < 4) {
            return "";
        }
        const std::string called_reference = lines[1];
        const std::string method_name = decode_target_name(lines[2]);
        const std::string return_sink = lines[3];
        auto& entry = instance_registry().at(called_reference);

        auto respond_method = [&]() -> ptip_ffi::Parameter {
            if (auto* point = std::any_cast<Point<int>>(&entry)) {
                if (method_name == "distance_to_origin") {
                    return ptip_ffi::encode_value(point->distance_to_origin());
                }
            }
            if (auto* point = std::any_cast<Point<double>>(&entry)) {
                if (method_name == "distance_to_origin") {
                    return ptip_ffi::encode_value(point->distance_to_origin());
                }
            }
            if (auto* list = std::any_cast<LinkedList<int>>(&entry)) {
                if (method_name == "append") {
                    const auto value = ptip_ffi::decode_parameter_line(lines[4]);
                    list->append(ptip_ffi::decode_value<int>(value));
                    return ptip_ffi::Parameter{ptip_ffi::ParameterKind::Boolean, "1"};
                }
                if (method_name == "get") {
                    const auto value = ptip_ffi::decode_parameter_line(lines[4]);
                    const std::size_t index = static_cast<std::size_t>(ptip_ffi::decode_value<double>(value));
                    return ptip_ffi::encode_value(list->get(index));
                }
            }
            if (auto* map = std::any_cast<HashMap<std::string, int>>(&entry)) {
                if (method_name == "set") {
                    const auto key = ptip_ffi::decode_parameter_line(lines[4]);
                    const auto value = ptip_ffi::decode_parameter_line(lines[5]);
                    const auto key_string = ptip_ffi::decode_value<std::string>(key);
                    const auto value_int = ptip_ffi::decode_value<int>(value);
                    map->set(key_string, value_int);
                    return ptip_ffi::Parameter{ptip_ffi::ParameterKind::Boolean, "1"};
                }
                if (method_name == "get") {
                    const auto key = ptip_ffi::decode_parameter_line(lines[4]);
                    return ptip_ffi::encode_value(map->get(ptip_ffi::decode_value<std::string>(key)));
                }
                if (method_name == "has") {
                    const auto key = ptip_ffi::decode_parameter_line(lines[4]);
                    return ptip_ffi::Parameter{ptip_ffi::ParameterKind::Boolean, map->has(ptip_ffi::decode_value<std::string>(key)) ? "1" : "0"};
                }
                if (method_name == "delete_key") {
                    const auto key = ptip_ffi::decode_parameter_line(lines[4]);
                    return ptip_ffi::Parameter{ptip_ffi::ParameterKind::Boolean, map->delete_key(ptip_ffi::decode_value<std::string>(key)) ? "1" : "0"};
                }
            }
            throw std::runtime_error("unsupported method: " + method_name);
        }();
        return respond_with_value(return_sink, respond_method);
    }

    if (kind == "D") {
        if (lines.size() >= 2) {
            instance_registry().erase(lines[1]);
        }
        return "";
    }

    return "";
}

{{DECLARATIONS}}

}  // namespace ptip_ffi_generated
