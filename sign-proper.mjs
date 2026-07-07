import { sign, verify, createPrivateKey, createPublicKey, generateKeyPairSync } from 'node:crypto';
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { randomBytes } from 'node:crypto';

// --- Config ---
const SETUP_PATH = 'D:\\桌面\\刷题宝\\src-tauri\\target\\release\\bundle\\nsis\\刷题宝_0.1.8_x64-setup.exe';
const KEYS_DIR = 'D:\\桌面\\刷题宝\\src-tauri\\keys';
const KEY_FILE = `${KEYS_DIR}\\updater-new-key.json`;
const APP_NAME = 'shuati-bao';

// --- Step 1: Load or generate key ---
let seed, keynum, pubKeyRaw, keynumHex;

if (existsSync(KEY_FILE)) {
  console.log(`Loading existing key from ${KEY_FILE}...`);
  const keyData = JSON.parse(readFileSync(KEY_FILE, 'utf-8'));
  seed = Buffer.from(keyData.seed, 'hex');
  keynum = Buffer.from(keyData.keynum, 'hex');
  pubKeyRaw = Buffer.from(keyData.publicKey, 'hex');
  keynumHex = keyData.keynumHex;
  console.log('Key loaded:');
  console.log('  keynum:', keynum.toString('hex'));
  console.log('  publicKey:', pubKeyRaw.toString('hex'));
} else {
  console.log('Generating new Ed25519 key pair...');
  const { privateKey, publicKey } = generateKeyPairSync('ed25519');
  const privKeyDer = privateKey.export({ type: 'pkcs8', format: 'der' });
  seed = privKeyDer.subarray(16); // 32-byte seed
  const pubKeyDer = publicKey.export({ type: 'spki', format: 'der' });
  pubKeyRaw = pubKeyDer.subarray(12); // 32-byte public key
  keynum = randomBytes(8);
  keynumHex = Buffer.from(keynum).reverse().toString('hex').toUpperCase();
  console.log('New key generated:');
  console.log('  seed:', seed.toString('hex'));
  console.log('  keynum:', keynum.toString('hex'));
}

// --- Reconstruct Node.js KeyObject from seed ---
// PKCS8 DER for Ed25519: 16-byte header + 32-byte seed
const PKCS8_HEADER = Buffer.from([
  0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06,
  0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20
]);
const pkcs8Der = Buffer.concat([PKCS8_HEADER, seed]);
const privateKey = createPrivateKey({ key: pkcs8Der, format: 'der', type: 'pkcs8' });
const publicKey = createPublicKey(privateKey);

// --- Step 2: Sign the setup.exe ---
console.log(`\nSigning ${SETUP_PATH}...`);
const setupContent = readFileSync(SETUP_PATH);
const rawSig = sign(null, setupContent, privateKey); // 64-byte Ed25519 signature
console.log('Raw signature (64 bytes):', rawSig.toString('hex'));

// --- Step 3: Construct 74-byte minisign signature ---
// Format: sig_alg(2) "Ed" + keynum(8) + sig(64)
const sigAlg = Buffer.from('Ed', 'ascii');
const minisignSig74 = Buffer.concat([sigAlg, keynum, rawSig]);
const sigBase64 = minisignSig74.toString('base64');
console.log('Minisign signature base64:', sigBase64);

// --- Step 4: Create trusted comment + global signature ---
// Trusted comment text (after "trusted comment: " prefix, 17 chars)
const timestamp = Math.floor(Date.now() / 1000);
const setupFileName = SETUP_PATH.split('\\').pop();
const trustedCommentText = `timestamp:${timestamp}\tfile:${setupFileName}`;
const trustedCommentLine = `trusted comment: ${trustedCommentText}`;

// Global signature = Ed25519_sign(raw_sig(64) + trusted_comment_text)
const globalSigInput = Buffer.concat([rawSig, Buffer.from(trustedCommentText, 'utf-8')]);
const globalSig = sign(null, globalSigInput, privateKey); // 64-byte
const globalSigBase64 = globalSig.toString('base64');
console.log('Trusted comment:', trustedCommentLine);
console.log('Global signature base64:', globalSigBase64);

// --- Step 5: Write 4-line .sig file (raw minisign format) ---
const untrustedCommentLine = `untrusted comment: ${APP_NAME} updater`;
const sigFileContent = `${untrustedCommentLine}\n${sigBase64}\n${trustedCommentLine}\n${globalSigBase64}\n`;
const sigPath = SETUP_PATH + '.sig';
writeFileSync(sigPath, sigFileContent);
console.log(`\n.sig file written to: ${sigPath}`);
console.log('--- .sig content ---');
console.log(sigFileContent);

// --- Step 6: Base64-encode the 4-line string for latest.json ---
// Tauri updater expects the signature field to be base64-encoded text
// that decodes to the full 4-line minisign signature
const signatureForLatestJson = Buffer.from(sigFileContent, 'utf-8').toString('base64');
console.log('\n=== signature for latest.json (base64-encoded) ===');
console.log(signatureForLatestJson);

// --- Step 7: Save key data (if new key) ---
if (!existsSync(KEY_FILE)) {
  const minisignPub = Buffer.concat([sigAlg, keynum, pubKeyRaw]);
  const pubBase64 = minisignPub.toString('base64');
  const pubKeyText = `untrusted comment: minisign public key: ${keynumHex}\n${pubBase64}\n`;
  const pubKeyBase64Encoded = Buffer.from(pubKeyText, 'utf-8').toString('base64');

  const keyData = {
    seed: seed.toString('hex'),
    keynum: keynum.toString('hex'),
    publicKey: pubKeyRaw.toString('hex'),
    keynumHex: keynumHex,
    pubKeyBase64Encoded: pubKeyBase64Encoded,
    pubKeyText: pubKeyText.trim(),
  };
  writeFileSync(KEY_FILE, JSON.stringify(keyData, null, 2));
  writeFileSync(`${KEYS_DIR}\\updater.key.pub`, pubKeyBase64Encoded);
  writeFileSync(`${KEYS_DIR}\\updater.key.pub.txt`, pubKeyText);
  console.log(`\nNew key saved to ${KEY_FILE}`);
  console.log('=== pubkey for tauri.conf.json ===');
  console.log(pubKeyBase64Encoded);
}

// --- Step 8: Verify ---
console.log('\n=== Verification ===');
const sigValid = verify(null, setupContent, publicKey, rawSig);
console.log('File signature valid:', sigValid ? 'PASS' : 'FAIL');

const globalSigValid = verify(null, globalSigInput, publicKey, globalSig);
console.log('Global signature valid:', globalSigValid ? 'PASS' : 'FAIL');

// Verify the minisign format parses back correctly
const parsedSig = Buffer.from(sigBase64, 'base64');
console.log('\nParsed sig length:', parsedSig.length, 'bytes (expected 74)');
console.log('Parsed sig_alg:', parsedSig.subarray(0, 2).toString('ascii'));
console.log('Parsed keynum:', parsedSig.subarray(2, 10).toString('hex'));
console.log('Keynum match:', parsedSig.subarray(2, 10).equals(keynum) ? 'YES' : 'NO');
