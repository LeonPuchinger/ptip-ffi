#pragma once

#include <cstdint>
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

std::string encode_base64_no_pad_utf8(const std::string& value);
std::string encode_parameter_line(const Parameter& parameter, const std::string* name = nullptr);
std::string serialize_invocation_path(const std::string& module_path, const std::string& callee_name);
void assert_no_crlf(const std::string& value);

}  // namespace ptip_ffi
