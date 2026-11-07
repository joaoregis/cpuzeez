# Security policy

## Supported versions

CPU ZeeZ 1.0 is in public beta (`1.0.0-beta.N`). Security fixes go into the newest beta; once 1.0.0 is out, into the latest release.

## Reporting a vulnerability

Please **don't open a public issue** for a security problem.

Use GitHub's private vulnerability reporting: on the repository, *Security › Report a vulnerability*. Include:

- the version (the *About* page) and the operating system;
- what you did, what happened, and what you expected;
- a sample file or command line if it involves an input (a capture, a report, an `--elevated-op` argument);
- whether it needs administrator rights.

You'll get an answer within a week. We'll say whether we agree it's a vulnerability, fix it in the next beta, and credit you in the CHANGELOG if you want.

## What counts

Highest priority:

- anything that makes the **elevated helper** do something outside the closed action list, or run without a prompt;
- a **parser** that crashes or misbehaves on a crafted file (captures, reports, settings, imported CSVs);
- a way to make the app **touch another process's memory** or inject into it (it must never);
- a flaw in the **update** check or the **bundled binaries'** verification.

The design and the controls are in [docs/security/threat-model.md](docs/security/threat-model.md).

## Not a vulnerability

- Releases being unsigned: known and documented ([ADR 0009](docs/adr/0009-releases-stay-unsigned-for-now.md)); verify downloads against `SHA256SUMS.txt`.
- Something that needs you to approve an administrator prompt for an action you chose.
- Something that needs an attacker who is already administrator.
