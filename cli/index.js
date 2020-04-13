const js = require("../pkg");
const fs = require('fs');
const readline = require('readline');

let file = "../migemo-compact-dict";
let buffer = fs.readFileSync(file);
let ab = new ArrayBuffer(buffer.length);
let view = new Uint8Array(ab);
buffer.copy(view);
let migemo = js.Migemo.new(view);
let mode_quiet = true;

const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    prompt: mode_quiet ? '' : 'QUERY: '
});

rl.prompt();

rl.on('line', (line) => {
    console.log((mode_quiet ? '' : 'PATTERN: ') + migemo.query(line.trim()));
    rl.prompt();
}).on('close', () => {
    process.exit(0);
});