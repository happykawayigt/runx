use runx_contracts::{Decision, Receipt, sha256_prefixed};
use serde::{Deserialize, Serialize};

use super::{ReceiptFinding, ReceiptFindingCode, ReceiptVerification, act_ids, verify_receipt};

/// The planner deliberation written beside the receipt and committed by
/// `lineage.journal_ref`. Moving the former `decisions[]` out of the signed body
/// must not weaken the `selected_act_id` integrity guarantee, so the journal is
/// hash-bound and verified through [`verify_receipt_with_journal`].
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptJournal {
    pub receipt_id: String,
    #[serde(default)]
    pub decisions: Vec<ReceiptJournalDecision>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptJournalDecision {
    #[serde(flatten)]
    pub decision: Decision,
}

impl ReceiptJournal {
    /// Canonical hash of this journal, used as `lineage.journal_ref` locator.
    #[must_use]
    pub fn digest(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        sha256_prefixed(json.as_bytes())
    }
}

pub fn validate_receipt_with_journal(
    receipt: &Receipt,
    journal: &ReceiptJournal,
) -> Result<(), ReceiptVerification> {
    let verification = verify_receipt_with_journal(receipt, journal);
    if verification.valid {
        Ok(())
    } else {
        Err(verification)
    }
}

/// Full verification: structural checks plus the journal-bound act-id integrity
/// property. A present-but-mismatched `journal_ref` is invalid (fail closed).
#[must_use]
pub fn verify_receipt_with_journal(
    receipt: &Receipt,
    journal: &ReceiptJournal,
) -> ReceiptVerification {
    let mut findings = verify_receipt(receipt).findings;
    let journal_ref = receipt
        .lineage
        .as_ref()
        .and_then(|lineage| lineage.journal_ref.as_ref());
    let Some(journal_ref) = journal_ref else {
        findings.push(ReceiptFinding {
            code: ReceiptFindingCode::JournalRefMissing,
            path: "lineage.journal_ref".to_owned(),
            message: "verify_with_journal requires a journal_ref to bind the journal".to_owned(),
        });
        return ReceiptVerification::from_findings(findings);
    };
    let expected = journal.digest();
    if journal_ref.locator.as_deref() != Some(expected.as_str()) {
        findings.push(ReceiptFinding {
            code: ReceiptFindingCode::JournalHashMismatch,
            path: "lineage.journal_ref.locator".to_owned(),
            message: "journal hash does not match lineage.journal_ref".to_owned(),
        });
        return ReceiptVerification::from_findings(findings);
    }
    let act_ids = act_ids(&receipt.acts);
    for (index, entry) in journal.decisions.iter().enumerate() {
        if let Some(act_id) = &entry.decision.selected_act_id {
            if !act_ids.contains(act_id) {
                findings.push(ReceiptFinding {
                    code: ReceiptFindingCode::DecisionSelectedActMissing,
                    path: format!("journal.decisions[{index}].selected_act_id"),
                    message: "selected act id must refer to an act in the receipt".to_owned(),
                });
            }
        }
    }
    ReceiptVerification::from_findings(findings)
}
