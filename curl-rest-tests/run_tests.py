#!/usr/bin/env python3
"""
Thin, simplified wrapper for running the high-performance Rust test runner.
Delegates all arguments to the Rust test_runner binary under cargo.
"""
import sys
import subprocess
import argparse

def main():
    parser = argparse.ArgumentParser(description="Centralized ICE REST Test Runner Wrapper")
    parser.add_argument("--group", default="ALL", help="Vaccine group to run (e.g. varicella, mmr, hepa, or ALL)")
    parser.add_argument("--case", default=None, help="Name of a single test case to run")
    parser.add_argument("--compare", action="store_true", help="Compare outputs of Java and Rust PoC")
    parser.add_argument("--record", action="store_true", help="Record Java responses as expected outputs")
    parser.add_argument("--verbose", "-v", action="store_true", help="Show detailed output for passing tests")
    
    args = parser.parse_args()

    # Determine command to run
    cmd = ["cargo", "run", "--release", "--bin", "test_runner", "--"]
    
    if args.record:
        cmd.extend(["--record", "tests/cases"])
    else:
        cmd.extend(["--run", "tests/cases"])
        if args.group.upper() != "ALL":
            # Strip cdsi_ prefix if it exists to match the Rust runner group naming
            group_name = args.group.upper()
            if group_name.startswith("CDSI_"):
                group_name = group_name[5:]
            cmd.extend(["--group", group_name])
        if args.case:
            cmd.extend(["--case", args.case])
        if args.compare:
            cmd.append("--compare")
        if args.verbose:
            cmd.append("--verbose")

    print(f"Executing: {' '.join(cmd)}")
    import os
    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    cargo_cwd = os.path.join(repo_root, "ice-rust-forecaster-poc")
    try:
        res = subprocess.run(cmd, cwd=cargo_cwd, check=True)
        sys.exit(res.returncode)
    except subprocess.CalledProcessError as e:
        sys.exit(e.returncode)

if __name__ == "__main__":
    main()
