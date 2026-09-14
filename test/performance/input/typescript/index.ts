export class HashMap<K, V> {
    private map: Map<K, V>;

    constructor() {
        this.map = new Map<K, V>();
    }

    set(key: K, value: V): void {
        this.map.set(key, value);
    }

    get(key: K): V | undefined {
        return this.map.get(key);
    }

    has(key: K): boolean {
        return this.map.has(key);
    }

    remove(key: K): boolean {
        return this.map.delete(key);
    }
}
