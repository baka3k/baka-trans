"""Immutable inputs for the Hy-MT2 feasibility gate and managed sidecar."""

MODEL_ID = "tencent/Hy-MT2-1.8B"
MODEL_REVISION = "9a341cd1b679d3efd23b46e847b01745a71ed792"
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
