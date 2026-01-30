#!/usr/bin/env node

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');

function checkCargoInstalled() {
  try {
    execSync('cargo --version', { stdio: 'pipe' });
    return true;
  } catch (error) {
    return false;
  }
}

function checkRustBinaryExists() {
  const platform = process.platform;
  const binaryName = platform === 'win32' ? 'grok.exe' : 'grok';
  const cargoHome = process.env.CARGO_HOME || path.join(os.homedir(), '.cargo');
  const binaryPath = path.join(cargoHome, 'bin', binaryName);

  return fs.existsSync(binaryPath);
}

function installViaCargo() {
  console.log('Installing grok-cli via cargo...');
  try {
    execSync('cargo install grok-cli', { stdio: 'inherit' });
    console.log('\n✓ grok-cli installed successfully!');
    return true;
  } catch (error) {
    console.error('Failed to install via cargo:', error.message);
    return false;
  }
}

function main() {
  console.log('Setting up grok-cli...\n');

  // Check if binary already exists
  if (checkRustBinaryExists()) {
    console.log('✓ grok binary already installed in ~/.cargo/bin');
    console.log('\nYou can now use the "grok" command!');
    return;
  }

  // Check if cargo is installed
  if (!checkCargoInstalled()) {
    console.log('⚠ Cargo (Rust package manager) not found.');
    console.log('\nTo use grok-cli, please install Rust:');
    console.log('  curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh');
    console.log('\nAfter installing Rust, run:');
    console.log('  cargo install grok-cli');
    console.log('\nAlternatively, install the prebuilt binary from:');
    console.log('  https://github.com/microtech/grok-cli/releases');
    process.exit(0); // Don't fail the npm install
  }

  // Try to install via cargo
  console.log('Attempting to install grok-cli binary...');
  const success = installViaCargo();

  if (!success) {
    console.log('\n⚠ Automatic installation failed.');
    console.log('\nPlease manually install:');
    console.log('  cargo install grok-cli');
    console.log('\nOr download from:');
    console.log('  https://github.com/microtech/grok-cli/releases');
    process.exit(0); // Don't fail the npm install
  }

  console.log('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log('  grok-cli is ready to use!');
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log('\nRun "grok --help" to get started.');
}

main();
