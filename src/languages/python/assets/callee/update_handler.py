    if isinstance(message, UpdateMessage):
        instance = _INSTANCE_REGISTRY[message.parent]
        setattr(instance, message.accessor, parameter_to_python(message.value))
        return AcknowledgeMessage(reference=message.acknowledgeSink)
