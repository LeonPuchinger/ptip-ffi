            case "/* {{ACCESSOR}} */": {
                const typedParent = parent as /* {{MODULE_NAMESPACE}} */./* {{TYPE_NAME}} */;
                return serializeValue(typedParent./* {{ACCESSOR}} */, "/* {{RETURN_KIND}} */", valueSink);
            }