from PyInstaller.utils.hooks import collect_all, collect_data_files, collect_submodules


vieneu_utils_datas, vieneu_utils_binaries, vieneu_utils_hidden = collect_all("vieneu_utils")
sea_g2p_datas, sea_g2p_binaries, sea_g2p_hidden = collect_all("sea_g2p")

datas = (
    collect_data_files("vieneu")
    + vieneu_utils_datas
    + sea_g2p_datas
)
binaries = vieneu_utils_binaries + sea_g2p_binaries
hiddenimports = (
    [
        "vieneu.base",
        "vieneu.v3turbo",
        "vieneu._v3_turbo_engine.onnx_runtime_lite",
    ]
    + collect_submodules("vieneu._v3_turbo_engine")
    + vieneu_utils_hidden
    + sea_g2p_hidden
)

a = Analysis(
    ["server.py"],
    pathex=[],
    binaries=binaries,
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=["gradio", "torch", "transformers", "librosa"],
    noarchive=False,
    optimize=1,
)
pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name="vieneu-bridge",
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
    name="vieneu-bridge",
)
