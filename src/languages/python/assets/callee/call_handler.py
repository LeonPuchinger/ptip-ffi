    if isinstance(message, CallMessage):
        target = _resolve_target(message.callee)
        constructor_reference = message.return_sink if _is_constructor_call(message.callee, target) else None
        return SendMessage(reference=message.return_sink, value=_invoke_target(target, message.positional_parameters, message.named_parameters, constructor_reference))
