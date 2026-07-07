import { generateKeyPairSync, sign, createPrivateKey, createPublicKey } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { randomBytes } from 'node:crypto';

const SETUP_PATH = 'D:\\桌面\\刷题宝\\src-tauri\\target\\release\\bundle\\nsis\\刷题宝_0.1.8_x64-setup.exe';
const KEYS_DIR = 'D:\\桌面\\刷题宝\\src-tauri\\keys';

// Step 1: Generate Ed25519 key pair
console.log('Generating Ed25519 key pair...');
const { privateKey, publicKey } = generateKeyPairSync('ed25519');

// Export raw key material
const privKeyDer = privateKey.export({ type: 'pkcs8', format: 'der' });
// PKCS8 Ed25519 private key: 16-byte header + 32-byte seed
const seed = privKeyDer.subarray(16);
console.log('Seed (32 bytes):', seed.toString('hex'));

const pubKeyDer = publicKey.export({ type: 'spki', format: 'der' });
// SPKI Ed25519 public key: 12-byte header + 32-byte public key
const pubKeyRaw = pubKeyDer.subarray(12);
console.log('Public key (32 bytes):', pubKeyRaw.toString('hex'));

// Step 2: Generate random keynum (8 bytes)
const keynum = randomBytes(8);
console.log('Keynum (8 bytes):', keynum.toString('hex'));

// Step 3: Sign the setup.exe
console.log('\nSigning setup.exe...');
const setupContent = readFileSync(SETUP_PATH);
const signature = sign(null, setupContent, privateKey);
console.log('Signature (64 bytes):', signature.toString('hex'));

// Step 4: Construct minisign signature (74 bytes)
// Format: sig_alg(2) "Ed" + keynum(8) + sig(64)
const sigAlg = Buffer.from('Ed', 'ascii');
const minisignSig = Buffer.concat([sigAlg, keynum, signature]);
console.log('\nMinisign signature (74 bytes):', minisignSig.length, 'bytes');
const sigBase64 = minisignSig.toString('base64');
console.log('Base64:', sigBase64);
console.log('Base64 length:', sigBase64.length, 'chars');

// Write .sig file
const sigPath = SETUP_PATH + '.sig';
const sigContent = `untrusted comment: shuati-bao updater\n${sigBase64}\n`;
writeFileSync(sigPath, sigContent);
console.log(`\n.sig file written to: ${sigPath}`);

// Step 5: Construct minisign public key (42 bytes)
// Format: sig_alg(2) "Ed" + keynum(8) + pk(32)
const minisignPub = Buffer.concat([sigAlg, keynum, pubKeyRaw]);
const pubBase64 = minisignPub.toString('base64');

// Keynum as big-endian hex for the comment
const keynumHex = Buffer.from(keynum).reverse().toString('hex').toUpperCase();
const pubKeyText = `untrusted comment: minisign public key: ${keynumHex}\n${pubBase64}\n`;
const pubKeyBase64Encoded = Buffer.from(pubKeyText, 'utf-8').toString('base64');

console.log('\n=== Public key for tauri.conf.json ===');
console.log('(base64-encoded, set as "pubkey" field):');
console.log(pubBase64);

// Step 6: Save private key for future use
// Store the raw seed + keynum so we can reconstruct the signing key later
const keyData = {
  seed: seed.toString('hex'),
  keynum: keynum.toString('hex'),
  publicKey: pubKeyRaw.toString('hex'),
  keynumHex: keynumHex,
  pubKeyBase64Encoded: pubKeyBase64Encoded,
  pubKeyText: pubKeyText.trim(),
};
writeFileSync(`${KEYS_DIR}\\updater-new-key.json`, JSON.stringify(keyData, null, 2));
console.log(`\nPrivate key data saved to: ${KEYS_DIR}\\updater-new-key.json`);

// Step 7: Write the new public key file (minisign text format, base64 encoded for Tauri)
writeFileSync(`${KEYS_DIR}\\updater.key.pub`, pubKeyBase64Encoded);
console.log(`Public key file updated: ${KEYS_DIR}\\updater.key.pub`);

// Also save the decoded text format for reference
writeFileSync(`${KEYS_DIR}\\updater.key.pub.txt`, pubKeyText);
console.log(`Public key text saved: ${KEYS_DIR}\\updater.key.pub.txt`);

// Step 8: Verify the signature
console.log('\n=== Verification ===');
const { verify } = await import('node:crypto');
const isValid = verify(null, setupContent, publicKey, signature);
console.log('Signature verification:', isValid ? 'PASS' : 'FAIL');

// Also verify the minisign format can be parsed back
const parsedSig = Buffer.from(sigBase64, 'base64');
console.log('Parsed sig length:', parsedSig.length, 'bytes (expected 74)');
console.log('Parsed sig_alg:', parsedSig.subarray(0, 2).toString('ascii'));
console.log('Parsed keynum:', parsedSig.subarray(2, 10).toString('hex'));
console.log('Parsed sig:', parsedSig.subarray(10, 74).toString('hex'));

const parsedPub = Buffer.from(pubBase64, 'base64');
console.log('\nParsed pub length:', parsedPub.length, 'bytes (expected 42)');
console.log('Parsed pub sig_alg:', parsedPub.subarray(0, 2).toString('ascii'));
console.log('Parsed pub keynum:', parsedPub.subarray(2, 10).toString('hex'));
console.log('Parsed pub key:', parsedPub.subarray(10, 42).toString('hex'));

console.log('\nKeynum match:', parsedSig.subarray(2, 10).equals(parsedPub.subarray(2, 10)) ? 'YES' : 'NO');
