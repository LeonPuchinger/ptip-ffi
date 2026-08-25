def {{NAME}}({{SIGNATURE}}) -> Any:
    message = CallMessage(module_path="", callee=FunctionTarget(name="{{NAME}}"), return_sink=str(uuid4()), positional=[{{POSITIONAL_ARGUMENTS}}], named={{{NAMED_ARGUMENTS}}})
    response = exchange(message)
    if isinstance(response, SendMessage):
        return parameter_to_python(response.value)
    if isinstance(response, ErrorMessage):
        raise RuntimeError(parameter_to_python(response.error))
    raise RuntimeError("Unexpected response from callee")
