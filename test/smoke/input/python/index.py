from math import sqrt
from typing import Generic, TypeVar


def trim_whitespace(str: str) -> str:
    return str.strip()


class Point:
    x: float
    y: float

    def __init__(self, x: float, y: float):
        self.x = x
        self.y = y

    def distance_to_origin(self) -> float:
        return sqrt(self.x * self.x + self.y * self.y)


def takes_point(point: Point) -> float:
    return point.distance_to_origin()


T = TypeVar("T")


class LinkedListNode(Generic[T]):
    value: T
    next: "LinkedListNode[T] | None"

    def __init__(
        self,
        value: T,
        next: "LinkedListNode[T] | None" = None,
    ):
        self.value = value
        self.next = next


class LinkedList(Generic[T]):
    root: LinkedListNode[T] | None

    def __init__(self):
        self.root = None

    def append(self, value: T) -> None:
        new_node = LinkedListNode(value)

        if self.root is None:
            self.root = new_node
            return

        current = self.root
        while current.next is not None:
            current = current.next

        current.next = new_node

    def get(self, index: int) -> T | None:
        current = self.root
        count = 0

        while current is not None:
            if count == index:
                return current.value

            count += 1
            current = current.next

        return None


K = TypeVar("K")
V = TypeVar("V")

class HashMap(Generic[K, V]):
    def __init__(self):
        self.map: dict[K, V] = {}

    def set(self, key: K, value: V) -> None:
        self.map[key] = value

    def get(self, key: K) -> V | None:
        return self.map.get(key)

    def has(self, key: K) -> bool:
        return key in self.map

    def remove(self, key: K) -> bool:
        if key in self.map:
            del self.map[key]
            return True
        return False
