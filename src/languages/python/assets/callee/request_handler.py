    if isinstance(message, RequestMessage):
        instance = _INSTANCE_REGISTRY[message.parent]
        return SendMessage(reference=message.valueSink, value=_result_to_parameter(getattr(instance, message.accessor)))
