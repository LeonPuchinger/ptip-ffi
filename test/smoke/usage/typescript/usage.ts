import { LinkedList, Point, trim_whitespace, HashMap, takes_point } from "./caller/index.ts";

const trimmed = trim_whitespace("  hello from typescript  ");
if (trimmed !== "hello from typescript") {
    throw new Error(`Unexpected trimmed value: ${String(trimmed)}`);
}

const point = new Point(3, 4);
const distance = point.distance_to_origin();
if (Math.abs(distance - 5) > 1e-9) {
    throw new Error(`Unexpected distance: ${String(distance)}`);
}

const distance2 = takes_point(point);
if (distance2 !== distance) {
    throw new Error(`Unexpected distance from takes_point: ${String(distance2)}`);
}

const values = new LinkedList<number>();
values.append(42);
if (values.get(0) !== 42) {
    throw new Error(`Unexpected list value: ${String(values.get(0))}`);
}

const map = new HashMap<string, number>();
map.set("answer", 42);
if (map.get("answer") !== 42 || !map.has("answer") || !map.remove("answer")) {
    throw new Error("Unexpected map result");
}

console.log("ts-cpp integration passed");
