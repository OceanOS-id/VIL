#!/usr/bin/env node
// Notification Builder — Node.js Sidecar (VIL SDK UDS+SHM with stdin/stdout fallback)
const path = require('path');

function buildNotification(data) {
    return {
        notification_id: `notif_${Date.now().toString(36)}`,
        title: data.title || 'Notification',
        body: data.body || '',
        channel: data.channel || 'general',
        priority: data.priority || 'normal',
        created_at: new Date().toISOString()
    };
}

if (process.env.VIL_SIDECAR_SOCKET) {
    // UDS+SHM mode
    const { SidecarApp } = require(path.join(__dirname, '../../../../crates/vil_sidecar/sdk/vil_sidecar_sdk.js'));
    const app = new SidecarApp('notification_builder');
    app.handler('execute', buildNotification);
    app.run();
} else {
    // Stdin/stdout line-delimited JSON (fallback)
    const readline = require('readline');
    const rl = readline.createInterface({ input: process.stdin });
    rl.on('line', line => {
        line = line.trim();
        if (!line) return;
        try {
            const data = JSON.parse(line);
            console.log(JSON.stringify(buildNotification(data)));
        } catch (e) {
            console.log(JSON.stringify({ error: e.message }));
        }
    });
}
