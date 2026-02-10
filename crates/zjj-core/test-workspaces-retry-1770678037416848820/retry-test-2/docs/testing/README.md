# Testing Lanes

This directory tracks test debt and first-run audit artifacts.

- Required lane (default, CI gate): `moon run :test`
- Research lane (DRQ/adversarial, non-gating): `moon run :test-research`

Use required lane results for merge readiness. Use research lane results for stabilization and investigation.
