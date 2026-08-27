class {{NAME}}:
    def __init__(self, *args: Any, **kwargs: Any) -> None:
        return_sink = uuid4().hex
        message = CallMessage(module_path="", callee=FunctionTarget(name="{{NAME}}"), return_sink=return_sink, positional_parameters=[{{POSITIONAL_ARGUMENTS}}], named_parameters={})
        bridge = establish_bridge()
        bridge.send(message)
        response = bridge.next_message()
        if response is None:
            raise RuntimeError("No response received from bridge")
        if isinstance(response, SendMessage):
            if response.reference != return_sink:
                raise RuntimeError("Mismatched UUID in response")
            if response.value.kind != "reference":
                raise RuntimeError("Unexpected message kind or value")
            self.uuid = str(parameter_to_python(response.value))
            weakref.finalize(self, _finalize_reference, self.uuid)
        elif isinstance(response, ErrorMessage):
            raise RuntimeError(parameter_to_python(response.error))
        else:
            raise RuntimeError("Unexpected response from callee")

    @classmethod
    def __from_reference(cls, uuid: str) -> "{{NAME}}":
        instance = cls.__new__(cls)
        instance.uuid = uuid
        weakref.finalize(instance, _finalize_reference, uuid)
        return instance

{{METHODS}}
