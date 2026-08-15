"""Immutable inputs for the Hy-MT2 feasibility gate and managed sidecar."""

MODEL_ID = "tencent/Hy-MT2-1.8B"
MODEL_REVISION = "9a341cd1b679d3efd23b46e847b01745a71ed792"
TARGET_LANGUAGE_NAME = "Vietnamese"
PROMPT_TEMPLATE = (
    "Translate the following text into {target_language}. Note that you should "
    "only output the translated result without any additional explanation:\n\n{source_text}"
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
    "LICENSE.txt",
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
    (".gitattributes", 1777, "561202ab7b2d5407a366be9410711a8243b1446b6af2d4318252f835c7eef79d"),
    ("LICENSE.txt", 11629, "1af3c6dc0c697277cbb6b68720787c1caa43a79c5626bf9f19cd8c00de9c8cd4"),
    ("README.md", 14763, "c81edecabcbf5c9f312680dd928485dd44830424986e42f450c52864babe5d81"),
    ("chat_template.jinja", 654, "b7491ec0e9c869dfce20f2176758099bf248d979dd05530ede99deb21698acee"),
    ("config.json", 1348, "da40c514cc74a5748a2e591b1b95fca4b7e94de05349abe4ea4164a82641de1a"),
    ("generation_config.json", 221, "0e28667f1cb4c7b880b9223b2d87978f88e79ed7ae037de1021f826c18d4ed6f"),
    ("model.safetensors", 4077072784, "29e9117a44c79f81857613601968ff482d8a23c2d6736a1710bba9e5ca4762e5"),
    ("special_tokens_map.json", 488, "bb9f59990034dae326581b9c62471523975417869f78a244b7ae2ce8cbb085eb"),
    ("tokenizer.json", 9527287, "b475bbef1b0b2fd57dcb865332b546475bd1ede2deb3bb91bafd0c047a8a530a"),
    ("tokenizer_config.json", 165815, "53bd8581b601a8ee9caefeb988207de50b3fc0b733295bdf5ad68dec4cc0b07c"),
)
TOTAL_MODEL_BYTES = sum(size for _, size, _ in MODEL_ARTIFACTS)
