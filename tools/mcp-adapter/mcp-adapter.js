#!/usr/bin/env node
/**
 * MCP stdio to HTTP adapter for Ribir debug server
 */

const http = require('http');
const readline = require('readline');

const RIBIR_DEBUG_PORT = process.env.RIBIR_DEBUG_PORT || 2333;
const RIBIR_MCP_URL = `http://127.0.0.1:${RIBIR_DEBUG_PORT}/mcp/message`;

// Disable output buffering
if (process.stdout._handle && process.stdout._handle.setBlocking) {
  process.stdout._handle.setBlocking(true);
}

const fs = require('fs');
const path = require('path');

// Load schema from shared JSON file
// First try same directory (installed location: ~/.ribir/), then fallback to repo path
let MCP_SCHEMA;
try {
  const localSchemaPath = path.join(__dirname, 'mcp_schema.json');
  const repoSchemaPath = path.join(__dirname, '../../core/src/debug_tool/mcp_schema.json');
  
  let schemaPath;
  if (fs.existsSync(localSchemaPath)) {
    schemaPath = localSchemaPath;
  } else if (fs.existsSync(repoSchemaPath)) {
    schemaPath = repoSchemaPath;
  } else {
    throw new Error('Schema file not found in ~/.ribir/ or repo location');
  }
  
  MCP_SCHEMA = JSON.parse(fs.readFileSync(schemaPath, 'utf8'));
} catch (e) {
  console.error('Failed to load MCP schema:', e);
  process.exit(1);
}

const ADAPTER_VERSION = MCP_SCHEMA.adapter_version || '0.0.0';
const FALLBACK_INIT_RESULT = MCP_SCHEMA.fallback_init_result;
const FALLBACK_TOOLS_RESULT = { tools: MCP_SCHEMA.tools };
const FALLBACK_RESOURCES_RESULT = { resources: MCP_SCHEMA.resources };

// Track if we've already shown the version warning
let versionWarningShown = false;

/**
 * Check version compatibility between adapter and server.
 * Returns a warning message if versions don't match, null otherwise.
 */
function checkVersionCompatibility(serverVersion) {
  if (versionWarningShown) return null;
  
  if (serverVersion && serverVersion !== ADAPTER_VERSION) {
    versionWarningShown = true;
    return `Version mismatch: adapter=${ADAPTER_VERSION}, server=${serverVersion}. ` +
           `Consider upgrading: run 'cli mcp upgrade'`;
  }
  return null;
}

// Process each request independently
async function processRequest(line) {
  try {
    const request = JSON.parse(line);

    // Skip notifications (no response needed)
    if (!request.id && request.method && request.method.startsWith('notifications/')) {
      return null;
    }

    // Try to connect with limited retries
    let attempt = 0;
    const maxAttempts = 3; // Retry a few times for startup race conditions
    
    while (true) {
        try {
            const response = await sendRequest(request);
            
            // Version check on successful initialize - add warning to response if mismatch
            if (request.method === 'initialize' && response.result?.serverInfo?.version) {
              const warning = checkVersionCompatibility(response.result.serverInfo.version);
              if (warning && response.result) {
                response.result._warning = warning;
              }
            }
            
            return JSON.stringify(response);
        } catch (err) {
            const isConnectionError = err.message.includes('ECONNREFUSED') || err.message.includes('Request failed');
            
            if (isConnectionError) {
                // FALLBACK MODE for lazy initialization
                const method = request.method;
                if (method === 'initialize') {
                    // Return fake capabilities so client thinks we are connected
                    return JSON.stringify({
                        jsonrpc: '2.0',
                        id: request.id,
                        result: FALLBACK_INIT_RESULT
                    });
                }
                if (method === 'tools/list') {
                     return JSON.stringify({
                        jsonrpc: '2.0',
                        id: request.id,
                        result: FALLBACK_TOOLS_RESULT
                    });
                }
                if (method === 'resources/list') {
                     return JSON.stringify({
                        jsonrpc: '2.0',
                        id: request.id,
                        result: FALLBACK_RESOURCES_RESULT
                    });
                }

                // For other methods (like tools/call), retry a few times then fail with helpful message
                if (attempt < maxAttempts) {
                    attempt++;
                    await new Promise(resolve => setTimeout(resolve, 1000));
                    continue;
                }
                
                // Return helpful error message to AI client
                const hint = `Cannot connect to Ribir debug server at port ${RIBIR_DEBUG_PORT}. ` +
                    `Make sure your Ribir app is running with the debug feature enabled: ` +
                    `cargo run -p your_app --features debug`;
                throw new Error(hint);
            }
            throw err;
        }
    }

  } catch (err) {
    return JSON.stringify({
      jsonrpc: '2.0',
      error: {
        code: -32603,
        message: err.message
      },
      id: JSON.parse(line).id || null
    });
  }
}

async function sendRequest(request) {
  return new Promise((resolve, reject) => {
    const postData = JSON.stringify(request);

    const req = http.request(RIBIR_MCP_URL, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(postData)
      }
    }, (res) => {
      let responseData = '';

      res.on('data', (chunk) => {
        responseData += chunk;
      });

      res.on('end', () => {
        try {
          resolve(JSON.parse(responseData));
        } catch (e) {
          reject(new Error(`Parse error: ${e.message}`));
        }
      });
    });

    req.on('error', (err) => {
      reject(new Error(`Request failed: ${err.message}`));
    });

    req.on('timeout', () => {
      req.destroy();
      reject(new Error('Request timeout'));
    });

    req.setTimeout(10000); // 10 second timeout
    req.write(postData);
    req.end();
  });
}

// Main processing loop
function main() {
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    terminal: false,
    crlfDelay: 100
  });

  (async () => {
    try {
      for await (const line of rl) {
        if (!line.trim()) continue;

        const response = await processRequest(line);
        if (response) {
          process.stdout.write(response + '\n');
          if (process.stdout.flush) {
            process.stdout.flush();
          }
        }
      }
    } catch (err) {
      // EOF or other error
      process.exit(0);
    }
  })();
}

main();
