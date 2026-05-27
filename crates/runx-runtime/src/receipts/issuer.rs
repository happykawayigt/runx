use runx_contracts::{ReceiptIssuer, ReceiptIssuerType};

pub(crate) fn local_runtime_issuer() -> ReceiptIssuer {
    local_issuer("runtime-skeleton", "sha256:runtime-skeleton-public")
}

pub(crate) fn local_target_runner_issuer() -> ReceiptIssuer {
    local_issuer(
        "target-runner-runtime",
        "sha256:target-runner-runtime-public",
    )
}

fn local_issuer(kid: &str, public_key_sha256: &str) -> ReceiptIssuer {
    ReceiptIssuer {
        issuer_type: ReceiptIssuerType::Local,
        kid: kid.to_owned().into(),
        public_key_sha256: public_key_sha256.to_owned().into(),
    }
}
