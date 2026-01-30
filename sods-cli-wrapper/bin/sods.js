#!/usr/bin/env node
const { spawn } = require('child_process');

// Check if Docker is available
const dockerCheck = spawn('docker', ['--version']);
dockerCheck.on('error', () => {
  console.error('Error: Docker is required but not found.');
  console.error('Please install Docker from https://docs.docker.com/get-docker/');
  process.exit(1);
});

// Spawn SODS Docker container with user arguments
const args = [
  'run', '--rm',
  '-v', `${process.cwd()}:/workspace`,
  '-w', '/workspace',
  'ghcr.io/logiccrafterdz/sods:latest',
  ...process.argv.slice(2)
];

const sods = spawn('docker', args, {
  stdio: 'inherit'
});

sods.on('error', (err) => {
  if (err.code === 'ENOENT') {
    console.error('Error: Docker command not found.');
    console.error('Please install Docker from https://docs.docker.com/get-docker/');
  } else {
    console.error('Error running SODS:', err.message);
  }
  process.exit(1);
});

sods.on('close', (code) => {
  process.exit(code);
});
