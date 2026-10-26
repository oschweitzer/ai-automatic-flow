# StageTrack

The stage track is the signature: one dot per stage of the ticket flow, filled up to the running stage.

It replaces a percentage bar because the flow has eight named stages (triage, plan, execute, open-mr, ci-gate, review, mr-loop, handover) read from the run's log.jsonl. Set `failed` when the current stage is blocked, which turns its ring red. Show it only for runs on this machine. A colleague's run shows "Run sur le poste de AL" instead, never an empty track.

The consumer provides `current`, `stage`, an optional `detail` like "depuis 12 min", and `failed`.
