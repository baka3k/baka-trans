# CI/CD audit

`Desktop checks` runs on every push and pull request across `windows-latest` and `macos-latest`. It installs Node 22 and stable Rust, runs npm install/build/test plus Rust tests. The Windows lane additionally produces and uploads an NSIS executable artifact.

Evidence: `.github/workflows/desktop.yml`.
