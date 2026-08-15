from PyInstaller.utils.hooks import collect_all, collect_submodules


torch_datas, torch_binaries, torch_hidden = collect_all("torch")
transformers_datas, transformers_binaries, transformers_hidden = collect_all("transformers")
safetensors_datas, safetensors_binaries, safetensors_hidden = collect_all("safetensors")

a = Analysis(
    ["poc_entry.py"],
    pathex=["."],
    binaries=torch_binaries + transformers_binaries + safetensors_binaries,
    datas=torch_datas + transformers_datas + safetensors_datas,
    hiddenimports=(
        torch_hidden
        + transformers_hidden
        + safetensors_hidden
        + collect_submodules("transformers.models.hunyuan_v1_dense")
        + ["hy_mt_poc.benchmark", "hy_mt_poc.download", "hy_mt_poc.runner"]
    ),
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=["accelerate", "tensorflow", "jax", "flax", "scipy"],
    noarchive=False,
    optimize=1,
)
pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name="hy-mt-poc",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    console=True,
)
coll = COLLECT(
    exe,
    a.binaries,
    a.datas,
    strip=False,
    upx=False,
    name="hy-mt-poc",
)
