            case "/* {{TYPE_NAME}} */": {
                const result = new /* {{MODULE_NAMESPACE}} */./* {{TYPE_NAME}} */(/* {{ARGUMENTS}} */);
                instanceRegistry.set(returnSink, result);
                return { kind: "reference", value: returnSink };
            }