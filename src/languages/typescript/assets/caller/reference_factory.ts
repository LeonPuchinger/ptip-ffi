    static __fromReference<T>(uuid: string): /* {{NAME}} */<T> {
        const reference = Object.create(/* {{NAME}} */.prototype) as /* {{NAME}} */<T> & { uuid: string };
        reference.uuid = uuid;
        finalizationRegistry.register(reference, uuid);
        return reference;
    }