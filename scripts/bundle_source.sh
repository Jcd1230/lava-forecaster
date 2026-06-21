#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_name="$(basename "$repo_root")"
vendor_dir="$repo_root/vendor"
temp_dir="$(mktemp -d)"
vendored_config_file="$temp_dir/config.toml"
dist_dir="$repo_root/dist"
archive_base="${repo_name}-source-bundle"
staging_dir="$dist_dir/$archive_base"
archive_path="$dist_dir/${archive_base}.zip"

cleanup() {
  rm -rf "$vendor_dir" "$staging_dir" "$temp_dir"
}

trap cleanup EXIT

mkdir -p "$dist_dir"

rm -rf "$vendor_dir" "$staging_dir"

echo "Vendoring Cargo dependencies into $vendor_dir"
cargo vendor "$vendor_dir" > "$vendored_config_file"

# Fix config.toml to use relative vendor path
sed -i 's|directory = ".*"|directory = "vendor"|' "$vendored_config_file"

# Create missing build/test vendor files and update Cargo checksum metadata
python3 - "$vendor_dir" << 'EOF'
import json
import hashlib
import pathlib
import sys

if len(sys.argv) < 2:
    print("Usage: script.py <vendor_dir>", file=sys.stderr)
    sys.exit(1)

vendor_dir = pathlib.Path(sys.argv[1])
if not vendor_dir.is_dir():
    print(f"Error: vendor directory {vendor_dir} does not exist", file=sys.stderr)
    sys.exit(1)

# Process cc crate
for p in vendor_dir.iterdir():
    if p.is_dir() and (p.name == "cc" or p.name.startswith("cc-")):
        (p / "gen").mkdir(parents=True, exist_ok=True)
        (p / "src").mkdir(parents=True, exist_ok=True)
        (p / "gen" / "cc_env.rs").touch()
        (p / "src" / "com.rs").touch()
        
        checksum = p / ".cargo-checksum.json"
        if checksum.exists():
            data = json.loads(checksum.read_text())
            files = data.get("files", {})
            updated = False
            for rel in list(files):
                filepath = p / rel
                if filepath.exists():
                    new_hash = hashlib.sha256(filepath.read_bytes()).hexdigest()
                    if files[rel] != new_hash:
                        files[rel] = new_hash
                        updated = True
            if updated:
                checksum.write_text(json.dumps(data, separators=(",", ":")))

# Process url crate
for p in vendor_dir.iterdir():
    if p.is_dir() and (p.name == "url" or p.name.startswith("url-")):
        (p / "tests").mkdir(parents=True, exist_ok=True)
        (p / "tests" / "expected_failures.txt").touch()
        
        checksum = p / ".cargo-checksum.json"
        if checksum.exists():
            data = json.loads(checksum.read_text())
            files = data.get("files", {})
            updated = False
            for rel in list(files):
                filepath = p / rel
                if filepath.exists():
                    new_hash = hashlib.sha256(filepath.read_bytes()).hexdigest()
                    if files[rel] != new_hash:
                        files[rel] = new_hash
                        updated = True
            if updated:
                checksum.write_text(json.dumps(data, separators=(",", ":")))
EOF

mkdir -p "$staging_dir"

echo "Collecting source tree into $staging_dir"
rsync -a \
  --exclude '/.antigravitycli/' \
  --exclude '/.git/' \
  --exclude '/.jj/' \
  --exclude '/.mise/' \
  --exclude '/.venv/' \
  --exclude '.DS_Store' \
  --exclude '/dist/' \
  --exclude '/pkg/' \
  --exclude '/target/' \
  --exclude '/*.log' \
  --exclude '/*_failures.txt' \
  "$repo_root/" "$staging_dir/"

mkdir -p "$staging_dir/.cargo"
cp "$vendored_config_file" "$staging_dir/.cargo/config.toml"

rm -f "$archive_path"

echo "Creating archive $archive_path"
(
  cd "$dist_dir"
  zip -rq "$(basename "$archive_path")" "$archive_base"
)

echo "Created $archive_path"
