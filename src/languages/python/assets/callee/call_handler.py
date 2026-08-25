    if isinstance(message, CallMessage):
        target = _resolve_target(message.callee)
        return SendMessage(reference=message.return_sink, value=_invoke_target(target, message.positional_parameters, message.named_parameters))
