# Code Signing Policy

Free code signing for the Windows builds of Fairstack Coreview is provided by [SignPath.io](https://signpath.io), with the certificate provided by the [SignPath Foundation](https://signpath.org).

## Team roles

- **Committers and reviewers:** [Ruslan Lendel](https://github.com/ruslanlendel)
- **Approvers:** [Ruslan Lendel](https://github.com/ruslanlendel). Every release must be approved manually before it is signed.

## What is signed

Only binaries built from this repository's source code by the GitHub Actions workflow `.github/workflows/build.yml` are submitted for signing. Third-party binaries are signed only if they are open source and built in the same workflow.

## Privacy

This program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it. See [PRIVACY.md](PRIVACY.md).

## Security

All team members use multi-factor authentication for GitHub and SignPath. Report vulnerabilities by opening a private security advisory in this repository.
