# Tester requests

Testers talk to the assistant through markdown files in this repo. The owner relays when
they've written, usually with one word: "update".

These rules win if they differ from any summary elsewhere.

## Threads

- One file per thread under `testers/<thread>/`, with that thread's traces and logs in its
  folder.
- "Close" moves the thread to the archive (`testers/archive/<thread>/`).

## Replies

- Reply in a `> [!tip] Assistant` box, below the tester's text.
- Plain language.
- Options spelled out as (a), (b), (c).
- No code until the tester says "go ahead".
- One system per version.
- Every reply says which 1:1 check you made (trace / KH1 decomp / exe) and what you couldn't
  check.

## Working several threads

- One sub-agent per waiting thread, each in its own worktree.

## Never

- Never write in a tester's box, post as a tester, or edit what a tester wrote.
- Never leave a waiting thread unanswered.

## Reply format

```markdown
> [!tip] Assistant
> What I found / what I changed, in plain words.
>
> Options:
> (a) ...
> (b) ...
>
> 1:1 check: checked <what> against <trace / KH1 decomp / exe>. Couldn't check <what>, because <why>.
```
