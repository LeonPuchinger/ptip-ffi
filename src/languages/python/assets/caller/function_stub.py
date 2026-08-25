def {{NAME}}({{SIGNATURE}}) -> Any:
    message = CallMessage(module_path="", callee=FunctionTarget(name="{{NAME}}"), return_sink=uuid4().hex, positional_parameters=[{{POSITIONAL_ARGUMENTS}}], named_parameters={})
    response = exchange(message)
    if isinstance(response, SendMessage):
        return parameter_to_python(response.value)
    if isinstance(response, ErrorMessage):
        raise RuntimeError(parameter_to_python(response.error))
    raise RuntimeError("Unexpected response from callee")
