                case "/* {{METHOD_NAME}} */": {
                    const typedInstance = instance as /* {{MODULE_NAMESPACE}} */./* {{TYPE_NAME}} */;
                    const result = typedInstance./* {{METHOD_NAME}} */(/* {{ARGUMENTS}} */);
                    return serializeValue(result, "/* {{RETURN_KIND}} */", returnSink);
                }