    if isinstance(message, MethodMessage):
        instance = _INSTANCE_REGISTRY[message.called_reference]
        target = getattr(instance, message.method_name)
        return SendMessage(reference=message.return_sink, value=_invoke_target(target, message.positional_parameters, message.named_parameters))
