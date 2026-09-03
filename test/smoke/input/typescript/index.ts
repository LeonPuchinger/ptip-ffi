export function trim_whitespace(str: string): string {
    return str.trim();
}

export class Point {
    x: number;
    y: number;

    constructor(x: number, y: number) {
        this.x = x;
        this.y = y;
    }

    distance_to_origin(): number {
        return Math.sqrt(this.x * this.x + this.y * this.y);
    }
}

export function takes_point(point: Point): number {
    return point.distance_to_origin();
}

class LinkedListNode<T> {
    value: T;
    next?: LinkedListNode<T>;

    constructor(value: T, next?: LinkedListNode<T>) {
        this.value = value;
        this.next = next;
    }
}

export class LinkedList<T> {
    root?: LinkedListNode<T>;

    constructor() {
        this.root = undefined;
    }

    append(value: T): void {
        const newNode = new LinkedListNode(value);
        if (!this.root) {
            this.root = newNode;
            return;
        }
        let current = this.root;
        while (current.next) {
            current = current.next;
        }
        current.next = newNode;
    }

    get(index: number): T | null {
        let current = this.root;
        let count = 0;
        while (current) {
            if (count === index) {
                return current.value;
            }
            count++;
            current = current.next;
        }
        return null;
    }
}

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
