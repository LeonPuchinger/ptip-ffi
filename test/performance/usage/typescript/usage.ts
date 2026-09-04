import * as fs from "node:fs";

import { HashMap } from "./caller/index.ts";

const operations = Number.parseInt(process.argv[2] ?? "", 10);
const resultsFile = process.argv[3];
if (!Number.isInteger(operations) || operations <= 0 || resultsFile === undefined) {
    console.error("Usage: usage.ts OPERATIONS RESULTS_FILE");
    process.exit(1);
}

const keyAlphabet = "abc";
const maxInsertValue = 1000;
const start = performance.now();
const map = new HashMap<string, number>();

for (let i = 0; i < operations; i++) {
    const keyLength = Math.floor(Math.random() * 2) + 1;
    let key = "";
    for (let j = 0; j < keyLength; j++) {
        key += keyAlphabet[Math.floor(Math.random() * keyAlphabet.length)];
    }
    const value = Math.floor(Math.random() * maxInsertValue);
    if (Math.random() < 0.5) {
        if (map.has(key)) {
            map.get(key);
        }
    } else {
        map.set(key, value);
    }
}

const elapsedMs = (performance.now() - start).toFixed(3);
const timestamp = localIsoTimestamp(new Date());
if (!fs.existsSync(resultsFile)) {
    fs.writeFileSync(resultsFile, "timestamp,operations,key_alphabet,max_insert_value,elapsed_ms\n");
}
fs.appendFileSync(resultsFile, `${timestamp},${operations},${keyAlphabet},${maxInsertValue},${elapsedMs}\n`);

function localIsoTimestamp(date: Date): string {
    const pad = (value: number, width = 2) => String(value).padStart(width, "0");
    const offsetMinutes = -date.getTimezoneOffset();
    const sign = offsetMinutes >= 0 ? "+" : "-";
    const absoluteOffset = Math.abs(offsetMinutes);
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
        + `T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}.${pad(date.getMilliseconds(), 3)}`
        + `${sign}${pad(Math.floor(absoluteOffset / 60))}:${pad(absoluteOffset % 60)}`;
}
