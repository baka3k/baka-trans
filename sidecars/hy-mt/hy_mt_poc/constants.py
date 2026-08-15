"""Immutable inputs for the Phase 09 candidate."""

MODEL_ID = "tencent/HY-MT1.5-1.8B"
MODEL_REVISION = "172d98efc7f534e05c86d3d49ed9d12d9c2a733b"
TARGET_LANGUAGE_NAME = "Vietnamese"
PROMPT_TEMPLATE = (
    "Translate the following segment into {target_language}, "
    "without additional explanation.\n\n{source_text}"
)

INFERENCE_FILES = (
    "chat_template.jinja",
    "config.json",
    "generation_config.json",
    "model.safetensors",
    "special_tokens_map.json",
    "tokenizer.json",
    "tokenizer_config.json",
)

REPOSITORY_FILES = (
    ".gitattributes",
    "License.txt",
    "README.md",
    *INFERENCE_FILES,
)

MAX_INPUT_CHARS = 4_000
MAX_NEW_TOKENS = 256
