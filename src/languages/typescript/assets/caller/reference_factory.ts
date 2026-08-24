    static __fromReference/* {{REFERENCE_TYPE_PARAMETERS}} */(uuid: string): /* {{NAME}} *//* {{TYPE_ARGUMENTS}} */ {
        const reference = Object.create(/* {{NAME}} */.prototype) as /* {{NAME}} *//* {{TYPE_ARGUMENTS}} */ & { uuid: string };
        reference.uuid = uuid;
        finalizationRegistry.register(reference, uuid);
        return reference;
    }