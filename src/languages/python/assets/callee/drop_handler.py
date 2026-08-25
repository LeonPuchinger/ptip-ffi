    if isinstance(message, DropMessage):
        _INSTANCE_REGISTRY.pop(message.reference, None)
        return None
