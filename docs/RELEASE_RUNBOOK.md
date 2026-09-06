# Release Runbook — Baka Trans

Quy trình release gộp cho cả macOS và Windows. Chi tiết nền tảng nằm ở
[RELEASE_GUIDE.md](RELEASE_GUIDE.md) (macOS) và [WINDOWS_RELEASE_GUIDE.md](WINDOWS_RELEASE_GUIDE.md) (Windows).

---

## 1. Tổng quan

| Nền tảng | Lệnh chính | Artifact | Tự động publish? |
| --- | --- | --- | --- |
| macOS (Apple Silicon) | `npm run release:check` → `npm run release:publish` | `.app` + `.dmg` | Có (tag + GitHub Release + upload DMG) |
| Windows | `npm run release:windows` | `.exe` (NSIS) + `.sha256` | Không (upload thủ công hoặc lấy từ CI) |
| CI (mọi push/PR) | [.github/workflows/desktop.yml](../.github/workflows/desktop.yml) | NSIS `.exe` làm artifact | Không (chỉ build kiểm tra) |

**Quy tắc vàng:** luôn build trên OS bạn định ship. macOS build trên Apple Silicon; Windows build trên Windows (script tự chặn nếu sai nền tảng).

Version phải khớp ở cả 5 nơi (lệnh `npm run version:set` cập nhật tất cả):

1. `package.json`
2. `package-lock.json`
3. `src-tauri/tauri.conf.json`
4. `src-tauri/Cargo.toml`
5. `src-tauri/Cargo.lock` (entry baka-trans)

---

## 2. Prerequisites (một lần setup)

```bash
# Node 22+, npm, Rust/Cargo + Tauri prerequisites
# GitHub CLI có quyền ghi repo:
gh auth login && gh auth status
```

**macOS — ký số + notarization:**

```bash
# Kiểm tra certificate (cần loại "Developer ID Application"):
security find-identity -v -p codesigning
```

Notarization — chọn MỘT trong hai bộ credential (env vars, không commit):

```bash
# Cách 1: Apple ID
export APPLE_ID="your-apple-id@example.com"
export APPLE_PASSWORD="app-specific-password"   # tạo tại appleid.apple.com
export APPLE_TEAM_ID="TEAMID"

# Cách 2: App Store Connect API
export APPLE_API_KEY="KEYID"
export APPLE_API_ISSUER="ISSUER_UUID"
export APPLE_API_KEY_PATH="/absolute/path/AuthKey_KEYID.p8"
```

Ghi đè signing identity khi cần (không sửa repo): `export APPLE_SIGNING_IDENTITY="Developer ID Application: ..."`.

---

## 3. Quy trình macOS (chuẩn)

### Bước 0 — Pre-flight

```bash
git switch main
git pull --ff-only origin main
npm ci
git status          # phải sạch hoàn toàn — publish sẽ từ chối nếu không
```

### Bước 1 — Bump version (bỏ qua nếu version đã được bump sẵn)

> `version:set` tự chặn nếu bạn không ở nhánh `main` hoặc working tree còn thay đổi.

```bash
npm run version:set -- 0.4.0
git diff                                     # review cả 5 nguồn
git add package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: prepare v0.4.0"
git push origin main
```

### Bước 2 — Build + verify (chưa public gì, có thể làm lại)

```bash
npm run release:check -- 0.4.0
```

Script tự động: chạy test → `tauri build` → verify chữ ký `.app`/`.dmg` → verify ticket notarization.

### Bước 3 — Publish (không thể hoàn tác)

```bash
npm run release:publish -- 0.4.0
```

Script sẽ **tự từ chối** khi: working tree chưa sạch · `main` ≠ `origin/main` · 5 nguồn version không khớp · tag/GitHub Release đã tồn tại · thiếu credential notarization (trừ khi có `--allow-unnotarized`).

Khi thành công, nó: push `main` (không force) → tạo + push tag `v0.4.0` → tạo GitHub Release với notes sinh từ tag trước → upload DMG kèm SHA-256 trong notes → đánh dấu **Latest**.

> **Một lệnh duy nhất** (gộp bước 2+3): `npm run release:macos -- all 0.4.0`
>
> **Release nội bộ có ký nhưng bỏ notarization:** thêm `--allow-unnotarized`. Người tải sẽ bị Gatekeeper cảnh báo — không hướng dẫn user tắt Gatekeeper.

### Bước 4 — Kiểm tra sau release

```bash
shasum -a 256 "src-tauri/target/release/bundle/dmg/Baka Trans_0.4.0_aarch64.dmg"   # so với SHA-256 trong release notes
```

Lưu ý: GitHub đổi khoảng trắng trong tên file thành dấu chấm — `Baka Trans_0.4.0_aarch64.dmg` thành `Baka.Trans_0.4.0_aarch64.dmg` trên URL tải xuống.

### Khắc phục sự cố

| Tình huống | Xử lý |
| --- | --- |
| Tag đã push nhưng tạo Release/upload asset lỗi | Sửa nguyên nhân ngoài (mạng, quyền `gh`) rồi `npm run release:publish -- 0.4.0 --resume` (thêm lại `--allow-unnotarized` nếu bản đó cố ý không notarize) |
| Release đã tồn tại nhưng thiếu asset | `gh release upload v0.4.0 "src-tauri/target/release/bundle/dmg/Baka Trans_0.4.0_aarch64.dmg#Baka Trans 0.4.0 — macOS Apple Silicon DMG"` |
| Sai version / cần release lại | Script **không bao giờ** xóa hay force-move tag. Chỉ hủy release khi cố ý rút bài; tuyệt đối không delete tag đã public |

---

## 4. Quy trình Windows

### Build local (trên máy Windows)

```powershell
git switch main
git pull --ff-only origin main
npm ci

npm run release:windows          # test (npm build/test + cargo test) → build sidecar VieNeu + Hy-MT → tauri build nsis → tạo file .sha256
npm run release:windows:check    # chỉ chạy kiểm tra, không build
```

Artifact: `src-tauri\target\release\bundle\nsis\*.exe` (+ `.sha256` đi kèm).

### Publish Windows (thủ công)

Windows chưa có script publish tự động như macOS:

```bash
gh release create v0.4.0 "src-tauri/target/release/bundle/nsis/Baka.Trans_0.4.0_x64-setup.exe" --notes "..."
```

(kèm file `.sha256` tương ứng nếu muốn). Có thể publish macOS trước rồi upload asset Windows vào cùng một release: `gh release upload v0.4.0 <file-exe>`.

### Lấy bản build từ CI (thay thế build local)

Mọi push/PR, [desktop.yml](../.github/workflows/desktop.yml) tự build NSIS trên `windows-latest` và upload artifact `baka-trans-windows-nsis`. Phù hợp cho bản test nội bộ; release chính thức vẫn nên build local để ký/notarize (nếu có cấu hình).

---

## 5. Checklist nhanh trước khi bấm publish

- [ ] Working tree sạch, `main` = `origin/main`
- [ ] `npm ci` đã chạy trên máy build
- [ ] Version khớp 5 nguồn (mở `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` so đối chiếu — script publish sẽ tự xác nhận lại)
- [ ] macOS: certificate Developer ID Application trong Keychain; env notarization đã export
- [ ] `npm run release:check` đã chạy sạch trước đó
- [ ] Tag `v0.4.0` chưa tồn tại (local lẫn remote)
- [ ] `gh auth status` OK

---

## 6. Vị trí artifact tóm tắt

```text
macOS:    src-tauri/target/release/bundle/macos/Baka Trans.app
          src-tauri/target/release/bundle/dmg/Baka Trans_<version>_aarch64.dmg
Windows:  src-tauri/target/release/bundle/nsis/*.exe (+ .sha256)
```
