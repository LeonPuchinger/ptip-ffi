    def {{NAME}}(self{{SIGNATURE}}) -> {{RETURN_TYPE}}:
        return_sink = uuid4().hex
        message = MethodMessage(called_reference=self.uuid, method_name="{{NAME}}", return_sink=return_sink, positional_parameters=[{{POSITIONAL_ARGUMENTS}}], named_parameters={})
        bridge = establish_bridge()
        bridge.send(message)
        response = bridge.next_message()
        if response is None:
            raise RuntimeError("No response received from bridge")
        if isinstance(response, SendMessage):
            if response.reference != return_sink:
                raise RuntimeError("Mismatched UUID in response")
            return parameter_to_python(response.value)
        if isinstance(response, ErrorMessage):
            raise RuntimeError(parameter_to_python(response.error))
        raise RuntimeError("Unexpected response from callee")