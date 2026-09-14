import * as fs from "node:fs";
import * as grpc from "@grpc/grpc-js";
import * as protoLoader from "@grpc/proto-loader";

const operations = Number.parseInt(process.argv[2] ?? "", 10);
const resultsFile = process.argv[3];
if (!Number.isInteger(operations) || operations <= 0 || resultsFile === undefined) process.exit(1);
const definition = protoLoader.loadSync(new URL("./hashmap.proto", import.meta.url).pathname);
const proto = grpc.loadPackageDefinition(definition) as any;
const client = new proto.hashmap.HashMapService(`127.0.0.1:${process.env.FFI_GRPC_PORT ?? 50051}`, grpc.credentials.createInsecure());
const call = (method: string, request: object): Promise<any> => new Promise((resolve, reject) => client[method](request, (error: Error | null, response: any) => error ? reject(error) : resolve(response)));
const keyAlphabet = "abc";
const maxInsertValue = 1000;
const handle = (await call("Create", {})).id;
const start = performance.now();
for (let i = 0; i < operations; i++) {
    const keyLength = Math.floor(Math.random() * 2) + 1;
    let key = "";
    for (let j = 0; j < keyLength; j++) key += keyAlphabet[Math.floor(Math.random() * keyAlphabet.length)];
    const value = Math.floor(Math.random() * maxInsertValue);
    if (Math.random() < 0.5) {
        if ((await call("Has", { handle, key })).value) await call("Get", { handle, key });
    } else {
        await call("Set", { handle, key, value });
    }
}
const elapsed = (performance.now() - start).toFixed(3);
if (!fs.existsSync(resultsFile)) fs.writeFileSync(resultsFile, "timestamp,operations,key_alphabet,max_insert_value,elapsed_ms\n");
fs.appendFileSync(resultsFile, `${localTimestamp()},${operations},${keyAlphabet},${maxInsertValue},${elapsed}\n`);
function localTimestamp(): string { const d = new Date(); const pad = (n: number, w = 2) => String(n).padStart(w, "0"); const o = -d.getTimezoneOffset(); const a = Math.abs(o); return `${d.getFullYear()}-${pad(d.getMonth()+1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}.${pad(d.getMilliseconds(),3)}${o >= 0 ? "+" : "-"}${pad(Math.floor(a/60))}:${pad(a%60)}`; }
