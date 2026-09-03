    if (kind == "C") {
        if (lines.size() < 3) return "";
        const std::string target = decode_target_name(lines[1]);
        const std::string return_sink = lines[2];
{{FUNCTION_CASES}}
        throw std::runtime_error("unknown function target: " + target);
    }

    if (kind == "M") {
        if (lines.size() < 4) return "";
        const std::string called_reference = lines[1];
        const std::string method_name = decode_target_name(lines[2]);
        const std::string return_sink = lines[3];
        auto& entry = instance_registry().at(called_reference);
{{METHOD_CASES}}
        throw std::runtime_error("unsupported method: " + method_name);
    }
