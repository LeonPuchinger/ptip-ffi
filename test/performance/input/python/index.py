from typing import Generic, TypeVar

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
