def {{NAME}}({{SIGNATURE}}) -> Any:
    message = CallMessage(module_path="", callee=FunctionTarget(name="{{NAME}}"), return_sink=uuid4().hex, positional_parameters=[{{POSITIONAL_ARGUMENTS}}], named_parameters={})
    bridge = establish_bridge()
    bridge.send(message)
    response = bridge.next_message()
    if response is None:
        raise RuntimeError("No response received from bridge")
    if isinstance(response, SendMessage):
        if response.reference != message.return_sink:
            raise RuntimeError("Mismatched UUID in response")
        return parameter_to_python(response.value)
    if isinstance(response, ErrorMessage):
        raise RuntimeError(parameter_to_python(response.error))
    raise RuntimeError("Unexpected response from callee")
