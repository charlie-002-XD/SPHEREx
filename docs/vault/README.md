# Context vault

Shared memory for every agent (Claude = decision maker, Devin = implementer). Read `state.md` and `decisions.md` before starting; write a handoff when you finish.

- `state.md`: where the project is right now (updated by Claude after each merge/decision).
- `decisions.md`: append-only decision log (D-001, D-002, ...). Never edit past entries; supersede them with a new one.
- `handoffs/<task-id>.md`: written by the agent that did a task: what was done, files touched, decisions taken, deviations from the task, open issues, commands to verify.
- `facts.md`: verified external facts with sources and date (IRSA endpoints, rules). Unverified claims go in `open-questions.md`, not here.
- `open-questions.md`: things nobody has verified yet, with the owner.

Rules: short, dated (UTC), English, no secrets, no raw logs. A vault entry that contradicts code or a newer entry must be fixed or superseded.
