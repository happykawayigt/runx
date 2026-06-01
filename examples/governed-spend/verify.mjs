#!/usr/bin/env node
//
// Independent receipt verifier. Uses ONLY the Node standard library (no runx,
// no third-party crypto). The point: you do not have to trust runx to believe a
// runx receipt. You recompute the canonical body hash yourself, and you verify
// the Ed25519 signature yourself, with a tool you already have.
//
// Usage:
//   node verify.mjs <receipt.json> [--pubkey <base64-raw-ed25519>] [--seed <base64-32-byte-seed>]
//
// If neither --pubkey nor --seed is given, the public demo seed is used (the
// same throwaway test key the demo signs with). For a real deployment you pass
// the issuer's published --pubkey and trust nothing else.
//
import crypto from "node:crypto";
import fs from "node:fs";

const DEMO_SEED_B64 = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI="; // public test key

function arg(name) {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : undefined;
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
  const raw = Buffer.from(rawB64, "base64");
  if (raw.length !== 32) throw new Error("pubkey must be 32 raw bytes");
  const spki = Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), raw]);
  return crypto.createPublicKey({ key: spki, format: "der", type: "spki" });
}

function rawPubBytes(keyObj) {
  return keyObj.export({ format: "der", type: "spki" }).subarray(-32);
}

const receiptPath = process.argv[2];
if (!receiptPath || receiptPath.startsWith("--")) {
  console.error("usage: node verify.mjs <receipt.json> [--pubkey <b64>] [--seed <b64>]");
  process.exit(2);
}
const r = JSON.parse(fs.readFileSync(receiptPath, "utf8"));

const pub = arg("--pubkey")
  ? publicKeyFromRaw(arg("--pubkey"))
  : publicKeyFromSeed(arg("--seed") ?? DEMO_SEED_B64);

const checks = [];

// 1. The named key matches the public key we are verifying with.
const ourKeyHash = "sha256:" + crypto.createHash("sha256").update(rawPubBytes(pub)).digest("hex");
checks.push(["issuer key matches the public key", ourKeyHash === r.issuer?.public_key_sha256,
  `issuer=${r.issuer?.public_key_sha256} ours=${ourKeyHash}`]);

// 2. The digest is the canonical hash of THIS receipt's body (content binding).
const body = { ...r }; delete body.signature; delete body.digest; delete body.metadata;
const recomputed = "sha256:" + crypto.createHash("sha256").update(canon(body), "utf8").digest("hex");
checks.push(["digest is the hash of the receipt body", recomputed === r.digest,
  `receipt=${r.digest} recomputed=${recomputed}`]);

// 3. The Ed25519 signature is valid over that digest.
let sigOk = false, sigDetail = "";
try {
  const sig = Buffer.from(r.signature.value.split("base64:")[1], "base64url");
  sigOk = sig.length === 64 && crypto.verify(null, Buffer.from(r.digest), pub, sig);
  sigDetail = `alg=${r.signature.alg} sigBytes=${sig.length}`;
} catch (e) { sigDetail = String(e); }
checks.push(["signature is valid over the digest", sigOk, sigDetail]);

let allPass = true;
console.log(`receipt: ${receiptPath}`);
console.log(`  id         : ${r.id}`);
console.log(`  disposition: ${r.seal?.disposition} (${r.seal?.reason_code})`);
console.log(`  issuer     : ${r.issuer?.type}/${r.issuer?.kid}`);
console.log("");
for (const [label, ok, detail] of checks) {
  allPass = allPass && ok;
  console.log(`  [${ok ? "PASS" : "FAIL"}] ${label}`);
  if (!ok) console.log(`         ${detail}`);
}
console.log("");
console.log(allPass
  ? "VERIFIED: runx signed exactly this receipt content. Verified with the Node standard library, trusting nothing from runx."
  : "NOT VERIFIED.");
process.exit(allPass ? 0 : 1);
