# LESSONS - auto-maintained by scripts/lessons.py

> Machine-owned. Do NOT hand-edit. Changes are overwritten on the next `lessons.py` write.
> Canonical state lives in `.specs/lessons.json`. Edit lessons only via the script.
> promote_threshold=2 distinct features · window_days=45 · quarantine_threshold=2

## Confirmed (load these at Plan/Checks)

Corroborated across multiple features. Safe to apply as guidance.

_none_

## Candidates (under observation - do NOT load as guidance yet)

Seen once or not yet corroborated. Tracked, not trusted.

### L-001 - A grep wiring proof must assert the forwarded argument at each call site, not count calls to the function.
- signal: `surviving_mutant` · recurrence: 1 feature(s) · scope: `proofs/static` · harmful: 0
- features: silence-controls
- evidence: static.sh live-decode / c14 (proofs/static)
- last seen: 2026-11-16T20:59:09Z

### L-002 - A criterion naming several subcommands needs a proof that runs every one of them.
- signal: `surviving_mutant` · recurrence: 1 feature(s) · scope: `omnivoice-cli` · harmful: 0
- features: silence-controls
- evidence: AC 19 / C23 (omnivoice-cli)
- last seen: 2026-11-16T20:59:10Z

## Quarantined (failed when applied - ignore)

A confirmed lesson that recurred alongside failure. Kept for the maintainer to review.

_none_
