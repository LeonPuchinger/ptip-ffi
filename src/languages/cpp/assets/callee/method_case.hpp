    if (auto* instance = std::any_cast<{{TYPE}}>(&entry)) {
        if (method_name == "{{NAME}}") {
{{BODY}}
        }
    }
