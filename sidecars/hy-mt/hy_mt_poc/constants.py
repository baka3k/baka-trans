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
PROTOCOL_VERSION = 1
RUNTIME_VERSION = "0.1.0"
MAX_PROTOCOL_LINE_BYTES = 64 * 1024
MAX_REQUEST_BYTES = 48 * 1024
DEFAULT_TRANSLATE_TIMEOUT_SECONDS = 20.0

# The install manifest deliberately includes only model inputs used at runtime.
# It is not a Hugging Face cache manifest and never accepts arbitrary repository
# files from a newer upstream revision.
MODEL_ARTIFACTS = (
    (".gitattributes", 1519, "11ad7efa24975ee4b0c3c3a38ed18737f0658a5f75a0a96787b576a78a023361"),
    ("License.txt", 16270, "d7d9db858500ac9073f4b5decef8e208454357226f535f65079ce4376047569f"),
    ("README.md", 8389, "302601d23ad541ef69827167e60d25c2c04265eff54fee690cb7c3c5638e7fcc"),
    ("chat_template.jinja", 654, "b7491ec0e9c869dfce20f2176758099bf248d979dd05530ede99deb21698acee"),
    ("config.json", 1342, "a1788df3224420f43ed1a424ad58bfacc34f689b0e477ce69d1298fa6d26292b"),
    ("generation_config.json", 221, "3586ba4829d9769b89523523cb562f2e894c519274f8a0e9b970287a0b1388a9"),
    ("model.safetensors", 4077072784, "07736f560253d8c991616060fb2d855420957c268fa7d32fa8593df2f83b21ab"),
    ("special_tokens_map.json", 488, "bb9f59990034dae326581b9c62471523975417869f78a244b7ae2ce8cbb085eb"),
    ("tokenizer.json", 9527287, "b475bbef1b0b2fd57dcb865332b546475bd1ede2deb3bb91bafd0c047a8a530a"),
    ("tokenizer_config.json", 165815, "53bd8581b601a8ee9caefeb988207de50b3fc0b733295bdf5ad68dec4cc0b07c"),
)
TOTAL_MODEL_BYTES = sum(size for _, size, _ in MODEL_ARTIFACTS)
