"""Immutable inputs for the managed offline translation models and sidecar."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class ModelArtifact:
    path: str
    size_bytes: int
    # Pinned sha256 of the file contents. None when the artifact lives in a
    # gated repository whose hashes are not public; integrity then falls back
    # to the pinned git blob sha1 (small files) or the exact size (LFS).
    sha256: str | None = None
    # Pinned git blob object id (sha1 over "blob <size>\\0" + content) from the
    # public repository tree; verifiable without repository access.
    git_blob_sha1: str | None = None


@dataclass(frozen=True)
class ModelSpec:
    key: str
    model_id: str
    revision: str
    prompt_style: str
    artifacts: tuple[ModelArtifact, ...]

    @property
    def paths(self) -> tuple[str, ...]:
        return tuple(artifact.path for artifact in self.artifacts)

    @property
    def total_bytes(self) -> int:
        return sum(artifact.size_bytes for artifact in self.artifacts)


HY_MT2_SPEC = ModelSpec(
    key="hy-mt2",
    model_id="tencent/Hy-MT2-1.8B",
    revision="9a341cd1b679d3efd23b46e847b01745a71ed792",
    prompt_style="hy_mt",
    artifacts=tuple(
        ModelArtifact(path, size, sha256)
        for path, size, sha256 in (
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
    ),
)

# Google's TranslateGemma lives in a manually gated repository: anonymous
# downloads fail and the LFS sha256 digests are not public. The install step
# therefore requires a Hugging Face token from an account that accepted the
# Gemma license. Integrity pins use the public commit, exact file sizes, and
# git blob ids for the small configuration files; the weights are size-checked
# only and their computed sha256 is recorded in the install manifest.
TRANSLATEGEMMA_4B_SPEC = ModelSpec(
    key="translategemma-4b",
    model_id="google/translategemma-4b-it",
    revision="10042cb0e6e7fdce748996a71dc3dc432a4e0c89",
    prompt_style="translategemma",
    artifacts=tuple(
        ModelArtifact(path, size, sha256, blob)
        for path, size, sha256, blob in (
            (".gitattributes", 1570, None, "52373fe24473b1aa44333d318f578ae6bf04b49b"),
            ("README.md", 17508, None, "44f3690778da1770bf19d64bace73480c5825bcc"),
            ("added_tokens.json", 35, None, "e17bde03d42feda32d1abfca6d3b598b9a020df7"),
            ("chat_template.jinja", 16982, None, "d571968f9770221512d29627697012202451983c"),
            ("config.json", 2618, None, "84ed56cf42ac645cf9e34f99548f0c1db6c2ee10"),
            ("generation_config.json", 156, None, "2fde3a7918391229df4dc54a3c57b7bed4fd332a"),
            ("model-00001-of-00002.safetensors", 4961251752, None, None),
            ("model-00002-of-00002.safetensors", 3639026128, None, None),
            ("model.safetensors.index.json", 90594, None, "63f4efcf7f2f61a304bde053a8c55b8a59eb7355"),
            ("preprocessor_config.json", 570, None, "cbd4f0cd77e39566f11921f9995bb3a4d008a83e"),
            ("processor_config.json", 70, None, "453c7966d4b5d0b4a317c585989f64c58c2a6bf0"),
            ("special_tokens_map.json", 662, None, "1a6193244714d3d78be48666cb02cdbfac62ad86"),
            ("tokenizer.json", 33384570, None, None),
            ("tokenizer.model", 4689074, None, None),
            ("tokenizer_config.json", 1155415, None, "ad0c34f8dd76c7138b35b3973f3b4a422d5136a9"),
        )
    ),
)

MODEL_SPECS = {spec.key: spec for spec in (HY_MT2_SPEC, TRANSLATEGEMMA_4B_SPEC)}
DEFAULT_MODEL_KEY = HY_MT2_SPEC.key


def get_model_spec(key: str) -> ModelSpec:
    try:
        return MODEL_SPECS[key]
    except KeyError:
        raise KeyError(f"Unknown offline translation model: {key}") from None


# Legacy aliases kept for the POC evidence tooling that predates the registry.
MODEL_ID = HY_MT2_SPEC.model_id
MODEL_REVISION = HY_MT2_SPEC.revision
MODEL_ARTIFACTS = tuple(
    (artifact.path, artifact.size_bytes, artifact.sha256) for artifact in HY_MT2_SPEC.artifacts
)
TOTAL_MODEL_BYTES = HY_MT2_SPEC.total_bytes
TARGET_LANGUAGE_NAME = "Vietnamese"
LANGUAGE_NAMES = {
    "af": "Afrikaans",
    "ar": "Arabic",
    "az": "Azerbaijani",
    "be": "Belarusian",
    "bg": "Bulgarian",
    "bn": "Bengali",
    "bs": "Bosnian",
    "ca": "Catalan",
    "cs": "Czech",
    "cy": "Welsh",
    "da": "Danish",
    "de": "German",
    "dz": "Dzongkha",
    "el": "Greek",
    "en": "English",
    "eo": "Esperanto",
    "es": "Spanish",
    "et": "Estonian",
    "eu": "Basque",
    "fa": "Persian",
    "fi": "Finnish",
    "fil": "Filipino",
    "fr": "French",
    "gl": "Galician",
    "gu": "Gujarati",
    "haw": "Hawaiian",
    "he": "Hebrew",
    "hi": "Hindi",
    "hr": "Croatian",
    "ht": "Haitian Creole",
    "hu": "Hungarian",
    "hy": "Armenian",
    "id": "Indonesian",
    "it": "Italian",
    "ja": "Japanese",
    "jv": "Javanese",
    "ka": "Georgian",
    "kk": "Kazakh",
    "ko": "Korean",
    "ku": "Kurdish",
    "la": "Latin",
    "lt": "Lithuanian",
    "lv": "Latvian",
    "mi": "Maori",
    "mk": "Macedonian",
    "ml": "Malayalam",
    "mn": "Mongolian",
    "ms": "Malay",
    "my": "Burmese",
    "ne": "Nepali",
    "nl": "Dutch",
    "nn": "Norwegian Nynorsk",
    "no": "Norwegian",
    "pa": "Punjabi",
    "pl": "Polish",
    "pt": "Portuguese",
    "pt-BR": "Brazilian Portuguese",
    "pt-PT": "European Portuguese",
    "ro": "Romanian",
    "ru": "Russian",
    "sk": "Slovak",
    "sl": "Slovenian",
    "sn": "Shona",
    "sq": "Albanian",
    "sr": "Serbian",
    "sv": "Swedish",
    "sw": "Swahili",
    "te": "Telugu",
    "th": "Thai",
    "tl": "Tagalog",
    "tr": "Turkish",
    "uk": "Ukrainian",
    "uz": "Uzbek",
    "vi": "Vietnamese",
    "yo": "Yoruba",
    "zh": "Chinese",
    "zh-Hans": "Simplified Chinese",
    "zh-Hant": "Traditional Chinese",
}
LANGUAGE_NAMES_ZH = {
    "ar": "阿拉伯语",
    "bn": "孟加拉语",
    "bo": "藏语",
    "cs": "捷克语",
    "de": "德语",
    "en": "英语",
    "es": "西班牙语",
    "fa": "波斯语",
    "fil": "菲律宾语",
    "fr": "法语",
    "gu": "古吉拉特语",
    "he": "希伯来语",
    "hi": "印地语",
    "id": "印尼语",
    "it": "意大利语",
    "ja": "日语",
    "kk": "哈萨克语",
    "km": "高棉语",
    "ko": "韩语",
    "mn": "蒙古语",
    "mr": "马拉地语",
    "ms": "马来语",
    "my": "缅甸语",
    "nl": "荷兰语",
    "pl": "波兰语",
    "pt": "葡萄牙语",
    "ru": "俄语",
    "ta": "泰米尔语",
    "te": "泰卢固语",
    "th": "泰语",
    "tl": "菲律宾语",
    "tr": "土耳其语",
    "ug": "维吾尔语",
    "uk": "乌克兰语",
    "ur": "乌尔多语",
    "vi": "越南语",
    "yue": "粤语",
    "zh": "中文",
    "zh-Hans": "简体中文",
    "zh-Hant": "繁体中文",
}
PROMPT_TEMPLATE_ZH = (
    "将以下文本翻译为{target_language}，注意只需要输出翻译后的结果，"
    "不要额外解释：\n\n{source_text}"
)
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
    "LICENSE.txt",
    "README.md",
    *INFERENCE_FILES,
)

MAX_INPUT_CHARS = 4_000
MAX_NEW_TOKENS = 256
PROTOCOL_VERSION = 1
RUNTIME_VERSION = "0.2.0"
TRUST_REMOTE_CODE = False
MAX_PROTOCOL_LINE_BYTES = 64 * 1024
MAX_REQUEST_BYTES = 48 * 1024
DEFAULT_TRANSLATE_TIMEOUT_SECONDS = 20.0

RUNTIME_IDENTITY = {
    "modelId": MODEL_ID,
    "revision": MODEL_REVISION,
    "protocolVersion": PROTOCOL_VERSION,
    "runtimeVersion": RUNTIME_VERSION,
    "trustRemoteCode": TRUST_REMOTE_CODE,
}

