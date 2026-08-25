class {{NAME}}:
    def __init__(self, *args: Any, **kwargs: Any) -> None:
        message = CallMessage(module_path="", callee=FunctionTarget(name="{{NAME}}"), return_sink=uuid4().hex, positional_parameters=[{{POSITIONAL_ARGUMENTS}}], named_parameters={})
        response = exchange(message)
        if isinstance(response, SendMessage):
            self.uuid = str(parameter_to_python(response.value))
        elif isinstance(response, ErrorMessage):
            raise RuntimeError(parameter_to_python(response.error))
        else:
            raise RuntimeError("Unexpected response from callee")

    @classmethod
    def __from_reference(cls, uuid: str) -> "{{NAME}}":
        instance = cls.__new__(cls)
        instance.uuid = uuid
        return instance
