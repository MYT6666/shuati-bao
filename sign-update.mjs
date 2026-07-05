import { createPrivateKey, sign } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';

const setupPath = process.argv[2];
const keyPath = process.argv[3];

// Read the private key file (base64 encoded minisign format)
const keyContent = readFileSync(keyPath, 'utf-8').trim();
const keyBase64 = keyContent.split('\n').pop().trim();
const keyBytes = Buffer.from(keyBase64, 'base64');

// Minisign key format: 
// First 2 bytes: R W (0x52, 0x57) for RSA, or R X for Ed25519
// Actually Tauri uses its own format. Let's extract the Ed25519 seed.
// The key starts with "RW" magic + algorithm ID + key ID + seed
// For Tauri's format: magic(2) + algorithm(1) + keyid(8) + seed(32) + checksum(32)

// Skip: RW(2) + algorithm(1) + keyid(8) = 11 bytes
// Seed starts at offset 11, length 32
const seed = keyBytes.subarray(11, 43);

// Create Ed25519 private key from seed
const privateKey = createPrivateKey({
  key: Buffer.concat([
    Buffer.from([0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20]),
    seed
  ]),
  format: 'der',
  type: 'pkcs8'
});

// Read the setup exe
const setupContent = readFileSync(setupPath);

// Sign with Ed25519
const signature = sign(null, setupContent, privateKey);
const sigBase64 = signature.toString('base64');

// Write the .sig file in Tauri's expected format
const sigPath = setupPath + '.sig';
const sigContent = `untrusted comment: shuati-bao updater\n${sigBase64}\n`;
writeFileSync(sigPath, sigContent);

console.log(`Signature written to ${sigPath}`);
console.log(`Signature: ${sigBase64.substring(0, 40)}...`);
