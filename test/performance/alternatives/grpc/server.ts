import * as grpc from "@grpc/grpc-js";
import * as protoLoader from "@grpc/proto-loader";
import crypto from "node:crypto";
import { pathToFileURL } from "node:url";

const packageDefinition = protoLoader.loadSync(new URL("./hashmap.proto", import.meta.url).pathname);
const proto = grpc.loadPackageDefinition(packageDefinition) as any;
const library = await import(pathToFileURL(`${process.env.GRPC_INPUT_ROOT}/input/typescript/index.ts`).href);
const instances = new Map<string, any>();
const service = {
    Create: (_call: any, callback: any) => {
        const id = crypto.randomUUID();
        instances.set(id, new library.HashMap<string, number>());
        callback(null, { id });
    },
    Set: (call: any, callback: any) => {
        instances.get(call.request.handle)!.set(call.request.key, Number(call.request.value));
        callback(null, {});
    },
    Get: (call: any, callback: any) => callback(null, { value: instances.get(call.request.handle)!.get(call.request.key) ?? 0 }),
    Has: (call: any, callback: any) => callback(null, { value: instances.get(call.request.handle)!.has(call.request.key) }),
};
const server = new grpc.Server();
server.addService(proto.hashmap.HashMapService.service, service);
const port = Number(process.env.FFI_GRPC_PORT ?? 50051);
server.bindAsync(`127.0.0.1:${port}`, grpc.ServerCredentials.createInsecure(), () => {
    console.log("ready");
    server.start();
});
