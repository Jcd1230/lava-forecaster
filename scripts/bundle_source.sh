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
