                case "/* {{METHOD_NAME}} */": {
                    const result = /* {{MODULE_NAMESPACE}} */./* {{TYPE_NAME}} */./* {{METHOD_NAME}} */(/* {{ARGUMENTS}} */);
                    return serializeValue(result, "/* {{RETURN_KIND}} */", returnSink);
                }