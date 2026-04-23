"""Unified dictionary pipeline orchestrator.

Consumes per-source `config.yaml` files under `sources/<category>/<key>/` and runs
the declared stages (extract, cleanup, frequency, poj, …) from `common.stages`.
Replaces the 8 per-source `run.sh` + ~70 numbered Python scripts that the refactor
inherited from the original layout.
"""
