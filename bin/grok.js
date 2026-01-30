#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

// Determine the correct binary name based on platform
function getBinaryPath() {
  const platform = process.platform;
  const arch = process.arch;

  let binaryName = 'grok';

  if (platform === 'win32') {
    binaryName = 'grok.exe';
  }

  // Look for the binary in several possible locations
  const possiblePaths = [
    // Installed via cargo in user's home
    path.join(require('os').homedir(), '.cargo', 'bin', binaryName),
    // Bundled with npm package
    path.join(__dirname, '..', 'bin', binaryName),
    path.join(__dirname, '..', 'target', 'release', binaryName),
    // In PATH
    binaryName
  ];

  for (const binPath of possiblePaths) {
    if (binPath === binaryName) {
      // Will try to find in PATH
      return binaryName;
    }
    if (fs.existsSync(binPath)) {
      return binPath;
    }
  }

  return binaryName; // Fallback to PATH lookup
}

function main() {
  const binaryPath = getBinaryPath();

  // Forward all arguments to the Rust binary
  const args = process.argv.slice(2);

  // Spawn the Rust binary
  const child = spawn(binaryPath, args, {
    stdio: 'inherit',
    shell: false
  });

  child.on('error', (err) => {
    if (err.code === 'ENOENT') {
      console.error('Error: grok binary not found.');
      console.error('\nPlease install the Rust binary first:');
      console.error('  cargo install grok-cli');
      console.error('\nOr make sure Rust and cargo are installed:');
      console.error('  curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh');
      process.exit(1);
    } else {
      console.error('Error executing grok:', err.message);
      process.exit(1);
    }
  });

  child.on('exit', (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
    } else {
      process.exit(code || 0);
    }
  });
}

main();
