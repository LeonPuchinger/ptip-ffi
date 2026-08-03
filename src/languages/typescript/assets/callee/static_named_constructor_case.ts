                case "/* {{METHOD_NAME}} */": {
                    const result = /* {{MODULE_NAMESPACE}} */./* {{TYPE_NAME}} */./* {{METHOD_NAME}} */(/* {{ARGUMENTS}} */);
                    instanceRegistry.set(returnSink, result);
                    return { kind: "reference", value: returnSink };
                }