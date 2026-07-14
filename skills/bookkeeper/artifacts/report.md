# Bookkeeper delivery report

- runx CLI version: `runx-cli 0.7.1`
- Publisher owner: `happykawayigt`
- Package name: `bookkeeper`
- Version: `0.1.0`
- Registry ref: `happykawayigt/bookkeeper@0.1.0`
- Public URL: https://runx.ai/x/happykawayigt/bookkeeper@0.1.0
- PR URL: https://github.com/runxhq/runx/pull/321
- Source URL: https://github.com/happykawayigt/runx/tree/7e439e0fc01b8f53e67ca077715b06ed91271fec/skills/bookkeeper
- Raw X.yaml: https://raw.githubusercontent.com/happykawayigt/runx/7e439e0fc01b8f53e67ca077715b06ed91271fec/skills/bookkeeper/X.yaml
- Raw SKILL.md: https://raw.githubusercontent.com/happykawayigt/runx/7e439e0fc01b8f53e67ca077715b06ed91271fec/skills/bookkeeper/SKILL.md
- Verification JSON: https://raw.githubusercontent.com/happykawayigt/runx/codex/frantic-bookkeeper/skills/bookkeeper/artifacts/verification.json
- Evidence JSON: https://raw.githubusercontent.com/happykawayigt/runx/codex/frantic-bookkeeper/skills/bookkeeper/artifacts/evidence.json
- Report URL: https://raw.githubusercontent.com/happykawayigt/runx/codex/frantic-bookkeeper/skills/bookkeeper/artifacts/report.md
- Publish method: `runx login --provider github --for publish`, then hosted registry publish to `https://api.runx.ai`
- Install command: `runx add happykawayigt/bookkeeper@0.1.0 --registry https://api.runx.ai`
- Hosted harness status: `passed`
- Harness cases: `clean-transactions-categorized` (sealed), `ambiguous-transaction-needs-review` (refused)
- Dogfood command: `runx skill happykawayigt/bookkeeper@0.1.0 --registry https://api.runx.ai --input-json transactions '[{"id":"frantic:receipt:247f05ba257f074d:worker-liability","date":"2026-07-12","description":"Frantic bounty 100 worker liability funded","amount":8,"currency":"USD"},{"id":"frantic:receipt:247f05ba257f074d:posting-fee","date":"2026-07-12","description":"Frantic bounty 100 demand-side posting fee","amount":-0.8,"currency":"USD"},{"id":"frantic:receipt:df458b36e0458cc9:worker-liability","date":"2026-07-12","description":"Frantic bounty 106 worker liability funded","amount":10,"currency":"USD"},{"id":"frantic:receipt:df458b36e0458cc9:posting-fee","date":"2026-07-12","description":"Frantic bounty 106 demand-side posting fee","amount":-1,"currency":"USD"}]' --input-json chart_of_accounts '[{"id":"2100","name":"Funded worker liability","type":"liability","keywords":["worker liability","liability funded"]},{"id":"6300","name":"Demand-side posting fees","type":"expense","keywords":["posting fee","demand-side fee"]}]' --input-json prior_period '{"transactions":[{"id":"frantic:receipt:6970b5626a70b6f5:worker-liability","date":"2026-07-05","description":"Frantic bounty 89 worker liability funded","amount":9,"currency":"USD","account_id":"2100"},{"id":"frantic:receipt:6970b5626a70b6f5:posting-fee","date":"2026-07-05","description":"Frantic bounty 89 demand-side posting fee","amount":-0.9,"currency":"USD","account_id":"6300"}]}' --skip-operator-context --receipt-dir ./dogfood-receipts --json`
- Receipt ref: `runx:receipt:sha256:457e8d1fa24f59e000be3702bd062b14b96b4613c353e9c92ff4fbe9ddd434e7`
- runx verify verdict: `valid=true`

## Real dogfood input

- The replayable input is embedded in `evidence.json.dogfood.input`.
- The input is derived from public Frantic funding receipts, not private bank or card data.
- Each `posting.funded` receipt becomes one worker-liability line and one demand-side posting-fee line.

## Result

- Categorized count: 4
- Anomaly count: 0
- Reconciliation matched: 4
- Reconciliation unmatched: 0
- Reconciliation total: 4
- Reconciliation debits: 1.8
- Reconciliation credits: 18
- Reconciliation net: 16.2
- Needs-review reason: ambiguous fixture refuses tied account evidence.
- Read-only boundary: the skill writes no ledger entries, calls no financial APIs, and emits only a reconciliation artifact.

## Reproduce

- Install: `runx add happykawayigt/bookkeeper@0.1.0 --registry https://api.runx.ai`
- Run: `runx skill happykawayigt/bookkeeper@0.1.0 --registry https://api.runx.ai --input-json transactions '[{"id":"frantic:receipt:247f05ba257f074d:worker-liability","date":"2026-07-12","description":"Frantic bounty 100 worker liability funded","amount":8,"currency":"USD"},{"id":"frantic:receipt:247f05ba257f074d:posting-fee","date":"2026-07-12","description":"Frantic bounty 100 demand-side posting fee","amount":-0.8,"currency":"USD"},{"id":"frantic:receipt:df458b36e0458cc9:worker-liability","date":"2026-07-12","description":"Frantic bounty 106 worker liability funded","amount":10,"currency":"USD"},{"id":"frantic:receipt:df458b36e0458cc9:posting-fee","date":"2026-07-12","description":"Frantic bounty 106 demand-side posting fee","amount":-1,"currency":"USD"}]' --input-json chart_of_accounts '[{"id":"2100","name":"Funded worker liability","type":"liability","keywords":["worker liability","liability funded"]},{"id":"6300","name":"Demand-side posting fees","type":"expense","keywords":["posting fee","demand-side fee"]}]' --input-json prior_period '{"transactions":[{"id":"frantic:receipt:6970b5626a70b6f5:worker-liability","date":"2026-07-05","description":"Frantic bounty 89 worker liability funded","amount":9,"currency":"USD","account_id":"2100"},{"id":"frantic:receipt:6970b5626a70b6f5:posting-fee","date":"2026-07-05","description":"Frantic bounty 89 demand-side posting fee","amount":-0.9,"currency":"USD","account_id":"6300"}]}' --skip-operator-context --receipt-dir ./dogfood-receipts --json`
- Verify: `runx verify --receipt dogfood-receipt.json --allow-local-development-signatures --json`
- Inspect `categorized[]`, `anomalies[]`, `reconciliation`, and `needs_review[]` before downstream use.
