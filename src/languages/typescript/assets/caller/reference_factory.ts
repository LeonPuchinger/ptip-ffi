    static __fromReference(uuid: string): /* {{NAME}} */ {
        const reference = Object.create(/* {{NAME}} */.prototype) as /* {{NAME}} */ & { uuid: string };
        reference.uuid = uuid;
        finalizationRegistry.register(reference, uuid);
        return reference;
    }