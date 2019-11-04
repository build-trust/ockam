const assert = require('assert');
const generate = require('..');

function test(value, message) {
  assert.ok(value, message);
  console.log(`\u001B[32m✓\u001B[39m ${message}`);
}

test(generate() === "a key", "key is generated")
