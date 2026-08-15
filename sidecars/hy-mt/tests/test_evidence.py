from __future__ import annotations

from pathlib import Path

from hy_mt_poc.evidence import sanitize_evidence


def test_evidence_redacts_home_and_hostname(monkeypatch) -> None:
    monkeypatch.setattr(Path, "home", classmethod(lambda cls: Path("/Users/private")))
    monkeypatch.setattr("hy_mt_poc.evidence.socket.gethostname", lambda: "work.example.test")
    assert sanitize_evidence(
        {"path": "/Users/private/project", "host": "work.example.test"}
    ) == {"path": "$HOME/project", "host": "$HOST"}
