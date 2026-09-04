import * as fs from "node:fs";
import { fileURLToPath } from "node:url";
import { HashMap } from "../../input/typescript/index.ts";

const OPERATIONS = parseInt(process.argv[2]) || -1;
if (OPERATIONS <= 0) {
    console.error("Please provide a positive integer for the number of operations.");
    process.exit(1);
}
const KEY_ALPHABET = "abc";
const MAX_INSERT_VALUE = 1000;

// A performance test that measures how long it takes to perform OPERATIONS
// of random read/write operations on a HashMap. For each repetition,
// the operation is chosen first: 50% write, 50% read. The key is a random string
// of 1-2 characters within the KEY_ALPHABET of characters, and the value is a
// random number between 0 and MAX_INSERT_VALUE.

const start = performance.now();

const map = new HashMap<string, number>();

for (let i = 0; i < OPERATIONS; i++) {
    const keyLength = Math.floor(Math.random() * 2) + 1;
    let key = "";
    for (let j = 0; j < keyLength; j++) {
        key += KEY_ALPHABET[Math.floor(Math.random() * KEY_ALPHABET.length)];
    }
    const value = Math.floor(Math.random() * MAX_INSERT_VALUE);
    if (Math.random() < 0.5) {
        if (map.has(key)) {
            map.get(key);
        }
    } else {
        map.set(key, value);
    }
}

const elapsed_ms = (performance.now() - start).toFixed(3);

const timestamp = localIsoTimestamp(new Date());
const line = `${timestamp},${OPERATIONS},${KEY_ALPHABET},${MAX_INSERT_VALUE},${elapsed_ms}\n`;
const resultsDir = fileURLToPath(
    new URL("../../results/", import.meta.url)
);
const resultsFile = fileURLToPath(
    new URL("../../results/native_ts.csv", import.meta.url)
);
if (!fs.existsSync(resultsDir)) {
    fs.mkdirSync(resultsDir, { recursive: true });
}
if (!fs.existsSync(resultsFile)) {
    fs.writeFileSync(resultsFile, "timestamp,operations,key_alphabet,max_insert_value,elapsed_ms\n");
}
fs.appendFileSync(resultsFile, line);

function localIsoTimestamp(date: Date): string {
    const pad = (value: number, width = 2) => String(value).padStart(width, "0");
    const offsetMinutes = -date.getTimezoneOffset();
    const sign = offsetMinutes >= 0 ? "+" : "-";
    const absoluteOffset = Math.abs(offsetMinutes);
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
        + `T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}.${pad(date.getMilliseconds(), 3)}`
        + `${sign}${pad(Math.floor(absoluteOffset / 60))}:${pad(absoluteOffset % 60)}`;
}
