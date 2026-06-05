import crypto from "node:crypto";

const DEMO_SEED_B64 = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

export function signedDemoReceipt(input) {
  const seed = Buffer.from(
    process.env.RUNX_RECEIPT_SIGN_ED25519_SEED_BASE64 || DEMO_SEED_B64,
    "base64",
  );
  if (seed.length !== 32) {
    throw new Error("RUNX_RECEIPT_SIGN_ED25519_SEED_BASE64 must decode to a 32-byte Ed25519 seed");
  }
  const privateKey = privateKeyFromSeed(seed);
  const publicKey = crypto.createPublicKey(privateKey);
  const publicKeyRaw = publicKey.export({ format: "der", type: "spki" }).subarray(-32);
  const body = {
    schema: "runx.receipt.v1",
    created_at: input.createdAt || new Date().toISOString(),
    name: input.name,
    seal: {
      disposition: input.disposition,
      reason_code: input.reasonCode,
    },
    issuer: {
      type: process.env.RUNX_RECEIPT_SIGN_ISSUER_TYPE || "hosted",
      kid: process.env.RUNX_RECEIPT_SIGN_KID || "runx-demo-key",
      public_key_sha256: sha256Prefixed(publicKeyRaw),
    },
    subject: input.subject,
  };
  const identifiedBody = {
    id: sha256Prefixed(canon(body)),
    ...body,
  };
  const digest = sha256Prefixed(canon(identifiedBody));
  const signature = crypto.sign(null, Buffer.from(digest), privateKey).toString("base64url");
  return {
    ...identifiedBody,
    digest,
    signature: {
      alg: "Ed25519",
      kid: body.issuer.kid,
      value: `base64:${signature}`,
    },
  };
}

export function sha256Prefixed(value) {
  return `sha256:${sha256Hex(value)}`;
}

export function sha256Hex(value) {
  return crypto.createHash("sha256").update(value).digest("hex");
}

export function canon(value) {
  if (value === null) return "null";
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "number" || typeof value === "string") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canon).join(",")}]`;
  return `{${Object.keys(value)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${canon(value[key])}`)
    .join(",")}}`;
}

function privateKeyFromSeed(seed) {
  const pkcs8 = Buffer.concat([Buffer.from("302e020100300506032b657004220420", "hex"), seed]);
  return crypto.createPrivateKey({ key: pkcs8, format: "der", type: "pkcs8" });
}
