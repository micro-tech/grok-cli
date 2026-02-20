#!/usr/bin/env node

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const os = require("os");

// Starlink-optimized retry configuration
const RETRY_CONFIG = {
  maxRetries: 3,
  baseDelay: 2000, // 2 seconds
  maxDelay: 60000, // 60 seconds
  timeout: 300000, // 5 minutes
};

/**
 * Sleep utility for retry delays
 */
function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Execute command with retry logic and exponential backoff
 * Optimized for Starlink network drops
 */
async function execWithRetry(command, options = {}, attempt = 1) {
  try {
    return execSync(command, {
      ...options,
      timeout: RETRY_CONFIG.timeout,
      stdio: options.stdio || "inherit",
    });
  } catch (error) {
    const isNetworkError =
      error.message.includes("timeout") ||
      error.message.includes("ETIMEDOUT") ||
      error.message.includes("ECONNRESET") ||
      error.message.includes("ENOTFOUND") ||
      error.message.includes("EAI_AGAIN") ||
      error.message.includes("network") ||
      error.code === "ETIMEDOUT" ||
      error.code === "ECONNRESET";

    if (isNetworkError && attempt < RETRY_CONFIG.maxRetries) {
      const delay = Math.min(
        RETRY_CONFIG.baseDelay * Math.pow(2, attempt - 1),
        RETRY_CONFIG.maxDelay,
      );

      console.log(
        `\n⚠ Network issue detected (attempt ${attempt}/${RETRY_CONFIG.maxRetries})`,
      );
      console.log(`Retrying in ${delay / 1000} seconds...`);

      await sleep(delay);
      return execWithRetry(command, options, attempt + 1);
    }

    throw error;
  }
}

function checkCargoInstalled() {
  try {
    execSync("cargo --version", { stdio: "pipe" });
    return true;
  } catch (error) {
    return false;
  }
}

function checkRustBinaryExists() {
  const platform = process.platform;
  const binaryName = platform === "win32" ? "grok.exe" : "grok";
  const cargoHome = process.env.CARGO_HOME || path.join(os.homedir(), ".cargo");
  const binaryPath = path.join(cargoHome, "bin", binaryName);

  return fs.existsSync(binaryPath);
}

async function installViaCargo() {
  console.log("Installing grok-cli via cargo...");
  console.log("(Network retries enabled for Starlink optimization)\n");

  try {
    await execWithRetry("cargo install grok-cli", { stdio: "inherit" });
    console.log("\n✓ grok-cli installed successfully!");
    return true;
  } catch (error) {
    console.error("Failed to install via cargo:", error.message);

    if (
      error.message.includes("timeout") ||
      error.message.includes("network")
    ) {
      console.error(
        "\n⚠ Network timeout - Starlink connection may have dropped.",
      );
      console.error("Please check your connection and try again.");
    }

    return false;
  }
}

async function main() {
  console.log("Setting up grok-cli v0.1.41...\n");

  // Check if binary already exists
  if (checkRustBinaryExists()) {
    console.log("✓ grok binary already installed in ~/.cargo/bin");
    console.log('\nYou can now use the "grok" command!');
    return;
  }

  // Check if cargo is installed
  if (!checkCargoInstalled()) {
    console.log("⚠ Cargo (Rust package manager) not found.");
    console.log("\nTo use grok-cli, please install Rust:");
    console.log(
      '  curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh',
    );
    console.log("\nAfter installing Rust, run:");
    console.log("  cargo install grok-cli");
    console.log("\nAlternatively, install the prebuilt binary from:");
    console.log("  https://github.com/microtech/grok-cli/releases");
    process.exit(0); // Don't fail the npm install
  }

  // Try to install via cargo with network retry support
  console.log("Attempting to install grok-cli binary...");
  const success = await installViaCargo();

  if (!success) {
    console.log("\n⚠ Automatic installation failed.");
    console.log("\nPlease manually install:");
    console.log("  cargo install grok-cli");
    console.log("\nOr download from:");
    console.log("  https://github.com/microtech/grok-cli/releases");
    process.exit(0); // Don't fail the npm install
  }

  console.log("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
  console.log("  grok-cli v0.1.41 is ready to use!");
  console.log("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
  console.log("\nNew features in v0.1.41:");
  console.log("  • External file access with audit logging");
  console.log("  • Tool loop debugging and diagnostics");
  console.log("  • Enhanced MCP server support");
  console.log("  • Improved network reliability");
  console.log('\nRun "grok --help" to get started.');
}

main().catch((error) => {
  console.error("Installation error:", error.message);
  process.exit(0); // Don't fail npm install
});
