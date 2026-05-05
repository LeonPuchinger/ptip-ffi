import { assertEquals } from "jsr:@std/assert";
import { MessageSocket } from "../socket.ts";

// dummy test
Deno.test("hello world", () => {
    assertEquals("Hello, Deno!", "Hello, Deno!");
});
