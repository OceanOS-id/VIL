#!/usr/bin/env node
// Chat Message Processor — Node.js Sidecar
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin });
let input = '';
rl.on('line', l => input += l);
rl.on('close', () => {
    const data = JSON.parse(input || '{}');
    const user = data.user || 'anonymous';
    const message = (data.message || '')
        .replace(/spam/gi, '***')
        .replace(/\bbad\b/gi, '***')
        .replace(/<script[^>]*>.*?<\/script>/gi, '[BLOCKED]');
    const result = { user, message, length: message.length, sanitized: true, timestamp: Date.now() };
    console.log(JSON.stringify(result));
});
