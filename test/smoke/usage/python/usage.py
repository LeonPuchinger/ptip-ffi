from caller.index import HashMap, LinkedList, Point, takes_point, trim_whitespace

trimmed = trim_whitespace("  hello from python  ")
assert trimmed == "hello from python", trimmed

point = Point(3, 4)
distance = point.distance_to_origin()
assert abs(distance - 5.0) < 1e-9, distance

distance2 = takes_point(point)
assert distance2 == distance, distance2

values = LinkedList()
values.append(42)
assert values.get(0) == 42

map = HashMap()
map.set("answer", 42)
assert map.get("answer") == 42
assert map.has("answer")
assert map.remove("answer")

print("Python caller integration passed")
