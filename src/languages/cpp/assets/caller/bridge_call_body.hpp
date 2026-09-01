{{PARAMETER_BINDINGS}}
    const std::string return_sink = ptip_ffi::generate_uuid();
    auto& bridge = ptip_ffi::establishBridge();
{{SEND_MESSAGE}}
    const std::string response = bridge.next_message();
    if (response.empty()) {
        throw std::runtime_error("No response received from the bridge");
    }
    const auto lines = ptip_ffi::split_message_lines(response);
    if (lines.size() < 3 || lines[0] != "S" || lines[1] != return_sink) {
        {{ERROR_HANDLING}}
    }
{{RETURN_STATEMENT}}
