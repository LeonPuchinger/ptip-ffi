#pragma once

#include <cctype>
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

inline std::uint64_t next_uuid_counter() {
    static std::uint64_t counter = 0;
    return ++counter;
}

inline std::string generate_uuid() {
    return "uuid-" + std::to_string(next_uuid_counter());
}

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

class ManagedReference {
public:
    ManagedReference(std::string uuid, Bridge* bridge) : uuid_(std::move(uuid)), bridge_(bridge) {}
    ~ManagedReference() {
        if (bridge_ != nullptr && !uuid_.empty()) {
            bridge_->send_message("D\n" + uuid_);
        }
    }

    ManagedReference(const ManagedReference&) = delete;
    ManagedReference& operator=(const ManagedReference&) = delete;

    ManagedReference(ManagedReference&& other) noexcept : uuid_(std::move(other.uuid_)), bridge_(other.bridge_) {
        other.bridge_ = nullptr;
        other.uuid_.clear();
    }

    ManagedReference& operator=(ManagedReference&& other) noexcept {
        if (this != &other) {
            if (bridge_ != nullptr && !uuid_.empty()) {
                bridge_->send_message("D\n" + uuid_);
            }
            uuid_ = std::move(other.uuid_);
            bridge_ = other.bridge_;
            other.bridge_ = nullptr;
            other.uuid_.clear();
        }
        return *this;
    }

    const std::string& uuid() const {
        return uuid_;
    }

private:
    std::string uuid_;
    Bridge* bridge_ = nullptr;
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

inline std::string decode_base64_no_pad_utf8(const std::string& value) {
    static const std::string alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    std::vector<int> table(256, -1);
    for (std::size_t i = 0; i < alphabet.size(); ++i) {
        table[static_cast<unsigned char>(alphabet[i])] = static_cast<int>(i);
    }

    std::string out;
    out.reserve(value.size() * 3 / 4 + 1);
    for (std::size_t i = 0; i < value.size(); i += 4) {
        const int a = table[static_cast<unsigned char>(value[i])];
        const int b = i + 1 < value.size() ? table[static_cast<unsigned char>(value[i + 1])] : 0;
        const int c = i + 2 < value.size() ? table[static_cast<unsigned char>(value[i + 2])] : 0;
        const int d = i + 3 < value.size() ? table[static_cast<unsigned char>(value[i + 3])] : 0;

        if (a < 0 || b < 0) {
            break;
        }

        const unsigned char byte0 = static_cast<unsigned char>((a << 2) | (b >> 4));
        out.push_back(static_cast<char>(byte0));
        if (i + 2 < value.size() && c >= 0) {
            const unsigned char byte1 = static_cast<unsigned char>(((b & 0x0F) << 4) | (c >> 2));
            out.push_back(static_cast<char>(byte1));
        }
        if (i + 3 < value.size() && d >= 0) {
            const unsigned char byte2 = static_cast<unsigned char>(((c & 0x03) << 6) | d);
            out.push_back(static_cast<char>(byte2));
        }
    }
    return out;
}

inline std::vector<std::string> split_message_lines(const std::string& message) {
    std::vector<std::string> lines;
    std::stringstream stream(message);
    std::string line;
    while (std::getline(stream, line)) {
        if (!line.empty()) {
            lines.push_back(line);
        }
    }
    return lines;
}

inline Parameter decode_parameter_line(const std::string& line) {
    if (line.empty()) {
        throw std::invalid_argument("empty parameter line");
    }
    const char kind = line[0];
    const std::string payload = line.size() > 1 ? line.substr(1) : std::string();
    switch (kind) {
        case 'i':
            return Parameter{ParameterKind::Integer, payload};
        case 'f':
            return Parameter{ParameterKind::Float, payload};
        case 'b':
            return Parameter{ParameterKind::Boolean, payload == "1" ? "1" : "0"};
        case 's':
            return Parameter{ParameterKind::String, decode_base64_no_pad_utf8(payload)};
        case 'r':
            return Parameter{ParameterKind::Reference, payload};
        default:
            throw std::invalid_argument("unknown parameter tag");
    }
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
