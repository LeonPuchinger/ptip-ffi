    if (target == "{{NAME}}") {
        const auto instance = {{INVOCATION}};
        instance_registry().emplace(return_sink, instance);
        return respond_with_reference(return_sink, return_sink);
    }
