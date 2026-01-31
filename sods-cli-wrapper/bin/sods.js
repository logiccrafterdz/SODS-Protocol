#!/usr/bin/env node
const { spawn } = require('child_process');
const path = require('path');
const os = require('os');

// Helper to provide OS-specific instructions
function getDockerInstallInstructions() {
  const platform = os.platform();
  if (platform === 'win32') return 'Windows: https://docs.docker.com/desktop/install/windows-install/';
  if (platform === 'darwin') return 'macOS: https://docs.docker.com/desktop/install/mac-install/';
  return 'Linux: https://docs.docker.com/engine/install/';
}

function handleError(err) {
  if (err.code === 'ENOENT') {
    console.error('\n❌ Error: Docker is not installed or not in your PATH.');
    console.error('SODS requires Docker to run the verification engine.');
    console.error('\nPlease install Docker for your platform:');
    console.error(getDockerInstallInstructions());
  } else {
    console.error('\n❌ Unexpected error running SODS:', err.message);
  }
  process.exit(1);
}

// 1. Initial Docker Check (optional but helpful for immediate feedback)
const dockerCheck = spawn('docker', ['--version']);
dockerCheck.on('error', handleError);

// 2. Prepare Docker arguments with robust path handling
const currentDir = process.cwd();
// On Windows, Docker Desktop requires specific path formatting for volume mounting
// but Node's path.resolve handles the absolute path appropriately.
const workspacePath = path.resolve(currentDir);

const args = [
  'run', '--rm',
  '-it', // Enable interactive mode for potential prompts
  '-v', `${workspacePath}:/workspace`,
  '-w', '/workspace',
  'ghcr.io/logiccrafterdz/sods:latest',
  ...process.argv.slice(2)
];

// 3. Launch SODS container
const sods = spawn('docker', args, {
  stdio: 'inherit',
  shell: os.platform() === 'win32' // Use shell on Windows for better compatibility
});

sods.on('error', handleError);

sods.on('close', (code) => {
  process.exit(code || 0);
});
