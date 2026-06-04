#!/usr/bin/env node
//
// Independent receipt verifier. Uses ONLY the Node standard library (no runx,
// no third-party crypto). The point: you do not have to trust runx to believe a
// runx receipt. You recompute the canonical body hash yourself, and you verify
// the Ed25519 signature yourself, with a tool you already have.
//
// Usage:
//   node verify.mjs <receipt.json> [--pubkey <base64-raw-ed25519>] [--seed <base64-32-byte-seed>]
//   node verify.mjs <receipt.json> [--jwks <jwks-file-or-url>] [--walk-ancestry] [--receipt-dir <dir>]
//
// If neither --pubkey nor --seed is given, the public demo seed is used (the
// same throwaway test key the demo signs with). For a real deployment you pass
// the issuer's published --pubkey and trust nothing else.
//
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const DEMO_SEED_B64 = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI="; // public test key
const MAX_ANCESTRY_DEPTH = 64;

function arg(name) {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

function hasFlag(name) {
  return process.argv.includes(name);
}

// runx.receipt.c14n.v1: strip the envelope's own signature/digest/metadata,
// then serialize with recursively sorted keys, compact, standard JSON escaping,
// integers as-is. (oss/crates/runx-receipts/src/canonical.rs)
function canon(v) {
  if (v === null) return "null";
  if (typeof v === "boolean") return v ? "true" : "false";
  if (typeof v === "number" || typeof v === "string") return JSON.stringify(v);
  if (Array.isArray(v)) return "[" + v.map(canon).join(",") + "]";
  return "{" + Object.keys(v).sort().map((k) => JSON.stringify(k) + ":" + canon(v[k])).join(",") + "}";
}

function publicKeyFromSeed(seedB64) {
  const seed = Buffer.from(seedB64, "base64");
  if (seed.length !== 32) throw new Error("seed must be 32 bytes");
  const pkcs8 = Buffer.concat([Buffer.from("302e020100300506032b657004220420", "hex"), seed]);
  const priv = crypto.createPrivateKey({ key: pkcs8, format: "der", type: "pkcs8" });
  return crypto.createPublicKey(priv);
}

function publicKeyFromRaw(rawB64) {
  return publicKeyFromRawBytes(Buffer.from(rawB64, "base64"));
}

function publicKeyFromRawBytes(raw) {
  if (raw.length !== 32) throw new Error("pubkey must be 32 raw bytes");
  const spki = Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), raw]);
  return crypto.createPublicKey({ key: spki, format: "der", type: "spki" });
}

function rawPubBytes(keyObj) {
  return keyObj.export({ format: "der", type: "spki" }).subarray(-32);
}

const receiptPath = process.argv[2];
if (!receiptPath || receiptPath.startsWith("--")) {
  console.error("usage: node verify.mjs <receipt.json> [--pubkey <b64>] [--seed <b64>] [--jwks <path-or-url>] [--walk-ancestry] [--receipt-dir <dir>]");
  process.exit(2);
}
let pub;
let walkAncestry = false;
let ancestryIndex = new Map();

async function resolvePublicKey(receipt) {
  if (arg("--pubkey")) return publicKeyFromRaw(arg("--pubkey"));
  if (arg("--jwks")) return publicKeyFromJwks(arg("--jwks"), receipt.issuer);
  return publicKeyFromSeed(arg("--seed") ?? DEMO_SEED_B64);
}

async function publicKeyFromJwks(locator, issuer) {
  const jwks = JSON.parse(await readText(locator));
  if (!Array.isArray(jwks.keys)) {
    throw new Error("JWKS must contain a keys array");
  }
  for (const key of jwks.keys) {
    if (key?.kty !== "OKP" || key?.crv !== "Ed25519" || typeof key?.x !== "string") continue;
    if (issuer?.kid && key.kid !== issuer.kid) continue;
    const raw = Buffer.from(key.x, "base64url");
    const keyHash = "sha256:" + crypto.createHash("sha256").update(raw).digest("hex");
    if (issuer?.public_key_sha256 && keyHash !== issuer.public_key_sha256) continue;
    return publicKeyFromRawBytes(raw);
  }
  throw new Error(`JWKS did not contain Ed25519 key ${issuer?.kid ?? "<unknown>"}`);
}

async function readText(locator) {
  if (locator.startsWith("http://") || locator.startsWith("https://")) {
    const response = await fetch(locator);
    if (!response.ok) throw new Error(`failed to fetch JWKS: ${response.status}`);
    return response.text();
  }
  if (locator.startsWith("file://")) return fs.readFileSync(new URL(locator), "utf8");
  return fs.readFileSync(locator, "utf8");
}

function sha256Prefixed(text) {
  return "sha256:" + crypto.createHash("sha256").update(text, "utf8").digest("hex");
}

function readReceipt(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function receiptBody(r) {
  const body = { ...r };
  delete body.signature;
  delete body.digest;
  delete body.metadata;
  return body;
}

function receiptIdentityBody(r) {
  const body = receiptBody(r);
  delete body.id;
  delete body.lineage;
  return body;
}

function childReceiptId(ref) {
  if (ref?.type !== "receipt" || typeof ref?.uri !== "string") return undefined;
  return ref.uri.startsWith("runx:receipt:") ? ref.uri.slice("runx:receipt:".length) : undefined;
}

function buildReceiptIndex(dir) {
  const index = new Map();
  for (const filePath of candidateReceiptFiles(dir)) {
    let value;
    try {
      value = readReceipt(filePath);
    } catch {
      continue;
    }
    collectReceipts(value, filePath, index);
  }
  return index;
}

function candidateReceiptFiles(dir) {
  const files = [];
  try {
    for (const name of fs.readdirSync(dir)) {
      if (name !== "index.json" && name.endsWith(".json")) files.push(path.join(dir, name));
    }
  } catch {
    return files;
  }
  return files;
}

function collectReceipts(value, sourcePath, index) {
  if (!value || typeof value !== "object") return;
  if (value.schema === "runx.receipt.v1" && typeof value.id === "string") {
    const bucket = index.get(value.id) ?? [];
    bucket.push({ receipt: value, sourcePath });
    index.set(value.id, bucket);
    return;
  }
  if (Array.isArray(value)) {
    for (const item of value) collectReceipts(item, sourcePath, index);
    return;
  }
  for (const child of Object.values(value)) collectReceipts(child, sourcePath, index);
}

function resolveIndexedReceipt(id) {
  if (!/^[A-Za-z0-9:_-]+$/.test(id)) return { status: "unsafe" };
  const matches = ancestryIndex.get(id) ?? [];
  if (matches.length === 0) return { status: "missing" };
  if (matches.length > 1) return { status: "ambiguous", matches };
  return { status: "found", ...matches[0] };
}

let allPass = true;

function check(label, ok, detail) {
  allPass = allPass && ok;
  console.log(`  [${ok ? "PASS" : "FAIL"}] ${label}`);
  if (!ok) console.log(`         ${detail}`);
}

function verifyOne(filePath, expectedParentId, depth, seen, receipt) {
  const r = receipt ?? readReceipt(filePath);
  const prefix = depth === 0 ? "receipt" : `child depth=${depth}`;
  console.log(`${prefix}: ${filePath}`);
  console.log(`  id         : ${r.id}`);
  console.log(`  disposition: ${r.seal?.disposition} (${r.seal?.reason_code})`);
  console.log(`  issuer     : ${r.issuer?.type}/${r.issuer?.kid}`);
  console.log("");

  if (seen.has(r.id)) {
    check("ancestry is acyclic", false, `cycle at ${r.id}`);
    return;
  }
  seen.add(r.id);

  // 1. The named key matches the public key we are verifying with.
  const ourKeyHash = "sha256:" + crypto.createHash("sha256").update(rawPubBytes(pub)).digest("hex");
  check("issuer key matches the public key", ourKeyHash === r.issuer?.public_key_sha256,
    `issuer=${r.issuer?.public_key_sha256} ours=${ourKeyHash}`);

  // 2. The digest is the canonical hash of THIS receipt's body (content binding).
  const recomputed = sha256Prefixed(canon(receiptBody(r)));
  check("digest is the hash of the receipt body", recomputed === r.digest,
    `receipt=${r.digest} recomputed=${recomputed}`);

  // 3. The id is content-addressed, independent of id/signature/digest/metadata/lineage.
  const recomputedId = sha256Prefixed(canon(receiptIdentityBody(r)));
  check("id is the hash of the receipt identity body", recomputedId === r.id,
    `receipt=${r.id} recomputed=${recomputedId}`);

  // 4. The Ed25519 signature is valid over that digest.
  let sigOk = false, sigDetail = "";
  try {
    const sig = Buffer.from(r.signature.value.split("base64:")[1], "base64url");
    sigOk = sig.length === 64 && crypto.verify(null, Buffer.from(r.digest), pub, sig);
    sigDetail = `alg=${r.signature.alg} sigBytes=${sig.length}`;
  } catch (e) { sigDetail = String(e); }
  check("signature is valid over the digest", sigOk, sigDetail);

  if (expectedParentId) {
    const actualParent = childReceiptId(r.lineage?.parent);
    check("child lineage parent points at the parent receipt", actualParent === expectedParentId,
      `parent=${actualParent ?? "<missing>"} expected=${expectedParentId}`);
  }

  const children = Array.isArray(r.lineage?.children) ? r.lineage.children : [];
  if (!walkAncestry) {
    console.log("");
    return;
  }
  check("ancestry depth stays bounded", depth < MAX_ANCESTRY_DEPTH, `depth=${depth}`);
  for (const [index, childRef] of children.entries()) {
    const childId = childReceiptId(childRef);
    check(`lineage child ${index} uses runx receipt ref`, Boolean(childId),
      `ref=${JSON.stringify(childRef)}`);
    if (!childId) continue;
    const expectedDigest = childRef.locator;
    const resolved = resolveIndexedReceipt(childId);
    check(`lineage child ${index} receipt resolves locally`, resolved.status === "found",
      `status=${resolved.status}`);
    if (resolved.status !== "found") continue;
    const child = resolved.receipt;
    check(`lineage child ${index} ref resolves by id`, child.id === childId,
      `child=${child.id} expected=${childId}`);
    if (expectedDigest) {
      check(`lineage child ${index} locator matches child digest`, child.digest === expectedDigest,
        `locator=${expectedDigest} child=${child.digest}`);
    }
    console.log("");
    verifyOne(resolved.sourcePath, r.id, depth + 1, seen, child);
  }
  console.log("");
}

async function main() {
  const rootPath = path.resolve(receiptPath);
  const rootReceipt = readReceipt(rootPath);
  pub = await resolvePublicKey(rootReceipt);
  walkAncestry = hasFlag("--walk-ancestry");
  const receiptDir = arg("--receipt-dir") ?? path.dirname(rootPath);
  ancestryIndex = walkAncestry ? buildReceiptIndex(receiptDir) : new Map();

  verifyOne(rootPath, undefined, 0, new Set(), rootReceipt);
  console.log("");
  console.log(allPass
    ? (walkAncestry
      ? "VERIFIED: runx signed this receipt tree. Ancestry was walked offline with the Node standard library, trusting nothing from runx."
      : "VERIFIED: runx signed exactly this receipt content. Verified with the Node standard library, trusting nothing from runx.")
    : "NOT VERIFIED.");
  process.exit(allPass ? 0 : 1);
}

await main();
