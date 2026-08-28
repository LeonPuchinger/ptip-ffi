#pragma once

#include <cstdint>
#include <cstdlib>
#include <map>
#include <sstream>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace ptip_ffi {

using UUID = std::string;

enum class ParameterKind { Integer, Float, Boolean, String, Reference };

struct Parameter {
    ParameterKind kind;
    std::string value;
};

struct CallMessage {
    std::string module_path;
    std::string callee_name;
    std::string return_sink;
    std::vector<Parameter> positional;
    std::map<std::string, Parameter> named;
};

struct MethodMessage {
    std::string called_reference;
    std::string method_name;
    std::string return_sink;
    std::vector<Parameter> positional;
    std::map<std::string, Parameter> named;
};

struct RequestMessage {
    std::string parent;
    std::string accessor;
    std::string value_sink;
};

struct UpdateMessage {
    std::string parent;
    std::string accessor;
    std::string acknowledge_sink;
    Parameter value;
};

struct SendMessage {
    std::string reference;
    Parameter value;
};

struct AcknowledgeMessage {
    std::string reference;
};

struct ErrorMessage {
    std::string reference;
    Parameter error;
};

struct DropMessage {
    std::string reference;
};

class Bridge {
public:
    explicit Bridge(class MessageSocket& socket);

    std::string next_message();
    void send_message(const std::string& payload);

private:
    class MessageSocket& socket_;
};

inline void assert_no_crlf(const std::string& value) {
    if (value.find('\r') != std::string::npos || value.find('\n') != std::string::npos) {
        throw std::invalid_argument("value contains newline characters");
    }
}

inline std::string encode_base64_no_pad_utf8(const std::string& value) {
    static const std::string alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    std::string out;
    out.reserve(((value.size() + 2) / 3) * 4);

    for (std::size_t i = 0; i < value.size(); i += 3) {
        const unsigned char b0 = static_cast<unsigned char>(value[i]);
        const unsigned char b1 = i + 1 < value.size() ? static_cast<unsigned char>(value[i + 1]) : 0;
        const unsigned char b2 = i + 2 < value.size() ? static_cast<unsigned char>(value[i + 2]) : 0;

        const unsigned char c0 = (b0 >> 2) & 0x3F;
        const unsigned char c1 = ((b0 & 0x03) << 4) | ((b1 >> 4) & 0x0F);
        const unsigned char c2 = ((b1 & 0x0F) << 2) | ((b2 >> 6) & 0x03);
        const unsigned char c3 = b2 & 0x3F;

        out.push_back(alphabet[c0]);
        out.push_back(alphabet[c1]);
        if (i + 1 < value.size()) {
            out.push_back(alphabet[c2]);
        } else {
            out.push_back('=');
        }
        if (i + 2 < value.size()) {
            out.push_back(alphabet[c3]);
        } else {
            out.push_back('=');
        }
    }

    while (!out.empty() && out.back() == '=') {
        out.pop_back();
    }
    return out;
}

inline std::string encode_parameter_line(const Parameter& parameter, const std::string* name = nullptr) {
    std::string main;
    switch (parameter.kind) {
        case ParameterKind::Integer: {
            std::int64_t value = std::stoll(parameter.value);
            std::stringstream stream;
            const std::uint64_t magnitude = value < 0 ? static_cast<std::uint64_t>(-(value + 1)) + 1ULL : static_cast<std::uint64_t>(value);
            stream << std::hex << std::nouppercase << magnitude;
            main = std::string("i") + (value < 0 ? "-" : "") + stream.str();
            break;
        }
        case ParameterKind::Float:
            main = std::string("f") + parameter.value;
            break;
        case ParameterKind::Boolean:
            main = std::string("b") + (parameter.value == "1" || parameter.value == "true" ? "1" : "0");
            break;
        case ParameterKind::String:
            assert_no_crlf(parameter.value);
            main = std::string("s") + encode_base64_no_pad_utf8(parameter.value);
            break;
        case ParameterKind::Reference:
            main = std::string("r") + parameter.value;
            break;
    }
    if (name == nullptr) {
        return main;
    }
    assert_no_crlf(*name);
    return main + " " + encode_base64_no_pad_utf8(*name);
}

inline std::string serialize_invocation_path(const std::string& module_path, const std::string& callee_name) {
    assert_no_crlf(module_path);
    assert_no_crlf(callee_name);
    if (module_path.empty()) {
        return encode_base64_no_pad_utf8(callee_name);
    }

    std::string encoded_module_path;
    std::size_t start = 0;
    while (start <= module_path.size()) {
        const std::size_t slash = module_path.find('/', start);
        const std::string component = module_path.substr(start, slash == std::string::npos ? std::string::npos : slash - start);
        if (!component.empty()) {
            if (!encoded_module_path.empty()) {
                encoded_module_path += "/";
            }
            encoded_module_path += encode_base64_no_pad_utf8(component);
        }
        if (slash == std::string::npos) {
            break;
        }
        start = slash + 1;
    }

    return encoded_module_path + "." + encode_base64_no_pad_utf8(callee_name);
}

}  // namespace ptip_ffi
