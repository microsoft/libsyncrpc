import test from 'ava'

import { SyncRpcChannel } from '../index.js';

test("should be able to send a message and get a response, synchronously.", t => {
  const channel = makeChannel();
  const response = channel.requestSync("echo", '"hello"');
  t.is(response, '"hello"');
  channel.murderInColdBlood();
});

test("should handle binary responses properly with requestBinarySync", t => {
  const channel = makeChannel();
  const buffer = channel.requestBinarySync("binary", "");
  
  // Verify it's a proper Buffer
  t.true(Buffer.isBuffer(buffer));
  
  // We expect 20 bytes (5 integers × 4 bytes each)
  t.is(buffer.length, 20);
  
  // Since we're getting a proper Buffer back, we can directly create a view
  const view = new Uint32Array(buffer.buffer, buffer.byteOffset, buffer.length / 4);
  
  // Verify the 5 integers are correct
  t.is(view[0], 1);          // First integer
  t.is(view[1], 10);         // Second integer (contains newline byte)
  t.is(view[2], 266);        // Third integer (contains newline byte)
  t.is(view[3], 1000000);    // Fourth integer
  t.is(view[4], 2147483647); // Fifth integer (max 32-bit signed int)

  // Ensure the channel is in a good state by making another request
  channel.requestBinarySync("binary", "");
  
  channel.murderInColdBlood();
});

test("can register a callback that will be requested by the child process before returning", t => {
  const channel = makeChannel();
  channel.registerCallback("echo", (_name, message) => message);
  const response = channel.requestSync("callback-echo", '"hello"');
  t.is(response, '"hello"');
});

test("callbacks are handled in the order in which they're requested", t => {
  const channel = makeChannel();
  channel.registerCallback("one", (_name, _message) => "one");
  channel.registerCallback("two", (_name, _message) => "two");
  channel.registerCallback("three", (_name, _message) => "three");
  const response = channel.requestSync("concat", "");
  t.is(response, '"onetwothree"');
  channel.murderInColdBlood();
});

test("throws if the child responds with an error", t => {
  const channel = makeChannel();
  t.throws(() => {
    channel.requestSync("error", "");
  }, { code: "GenericFailure", message: '"something went wrong"' });
  channel.murderInColdBlood();
});

test("throws if a callback throws", t => {
  const channel = makeChannel();
  channel.registerCallback("throw", () => { throw new Error("callback error") });
  t.throws(() => {
    channel.requestSync("throw", "");
  }, { code: "GenericFailure", message: /callback error/ });
  channel.murderInColdBlood();
});

test("should combine callback handling with binary response", t => {
  const channel = makeChannel();
  
  // Register a callback that will provide a suffix value to append to the binary data
  channel.registerCallback("getSuffix", (_name, _payload) => {
    // Return a magic number (42) that will be appended to the binary response
    return "42";
  });
  
  // Request binary data with callback interaction
  const buffer = channel.requestBinarySync("binary-with-callback", "5");
  
  // Verify it's a proper Buffer
  t.true(Buffer.isBuffer(buffer));
  
  // We expect 24 bytes (6 integers × 4 bytes each)
  // 5 integers from the sequence + 1 integer from the callback
  t.is(buffer.length, 24);
  
  // Create a view to check the integers
  const view = new Uint32Array(buffer.buffer, buffer.byteOffset, buffer.length / 4);
  
  // Verify the first 5 integers follow the sequence 1-5
  for (let i = 0; i < 5; i++) {
    t.is(view[i], i + 1); // 1, 2, 3, 4, 5
  }
  
  // Verify the final integer is 42 (our magic number from the callback)
  t.is(view[5], 42);
  
  channel.murderInColdBlood();
});

function makeChannel() {
  return new SyncRpcChannel("cargo", ["run", "--release", "--example", "socket_child"]);
}