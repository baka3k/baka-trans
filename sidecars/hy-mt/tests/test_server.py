from __future__ import annotations

import io
import json
import threading
import time

import pytest

from hy_mt_poc.runner import TranslationResult
from hy_mt_poc.lifecycle import LifecycleError
from hy_mt_poc.server import ServeLoop, hardened_environment


class FakeRunner:
    def metadata(self):
        return {"actualDevice": "mps:0", "actualDtype": "bfloat16", "modelLoadMs": 12.5}

    def translate(self, text, **kwargs):
        cancellation = kwargs["cancellation"]
        for _ in range(100):
            if cancellation.is_set():
                return TranslationResult("", 1, 0, 1.0, 0.0, "greedy", True, {})
            time.sleep(0.001)
        return TranslationResult("xin chào", 1, 2, 10.0, 200.0, "greedy", False, {})


def frames(output: io.StringIO) -> list[dict[str, object]]:
    return [json.loads(line) for line in output.getvalue().splitlines()]


def test_ready_and_translation_result_use_protocol_only_stdout() -> None:
    output = io.StringIO()
    loop = ServeLoop(FakeRunner(), output)
    loop.ready()
    loop._handle({"type": "translate", "protocolVersion": 1, "id": "a", "sourceLanguage": "ja", "targetLanguage": "vi", "text": "こんにちは"})
    assert [frame["type"] for frame in frames(output)] == ["ready", "result"]


def test_reader_delivers_cancel_while_worker_is_generating() -> None:
    output = io.StringIO()
    loop = ServeLoop(FakeRunner(), output)
    request = {"type": "translate", "protocolVersion": 1, "id": "a", "sourceLanguage": "ja", "targetLanguage": "vi", "text": "こんにちは"}
    worker = threading.Thread(target=loop._handle, args=(request,))
    worker.start()
    time.sleep(0.01)
    loop.receive(io.BytesIO(b'{"type":"cancel","protocolVersion":1,"id":"a"}\n'))
    worker.join(timeout=1)
    assert not worker.is_alive()
    assert frames(output)[0] == {"type": "cancelled", "id": "a", "active": True}


def test_parent_pipe_eof_stops_loop() -> None:
    output = io.StringIO()
    loop = ServeLoop(FakeRunner(), output)
    loop.messages.put(None)
    loop.run()


def test_serve_rejects_hub_credentials(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("HF_TOKEN", "must-not-be-used")
    with pytest.raises(LifecycleError, match="credentials"):
        hardened_environment()
