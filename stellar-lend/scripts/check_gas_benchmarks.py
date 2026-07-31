#!/usr/bin/env python3
import argparse
import json
import sys


def index_benchmarks(items):
    return {(item["operation"], item["scenario"]): item for item in items}


def main():
    parser = argparse.ArgumentParser(description="Compare current gas benchmarks against baseline.")
    parser.add_argument("--baseline", required=True, help="Path to baseline benchmark JSON")
    parser.add_argument("--current", required=True, help="Path to current benchmark JSON")
    parser.add_argument(
        "--max-regression-pct",
        type=float,
        default=10.0,
        help="Maximum allowed regression percentage",
    )
    args = parser.parse_args()

    with open(args.baseline, "r", encoding="utf-8") as f:
        baseline = json.load(f)
    with open(args.current, "r", encoding="utf-8") as f:
        current = json.load(f)

    baseline_ix = index_benchmarks(baseline.get("benchmarks", []))
    current_ix = index_benchmarks(current.get("benchmarks", []))

    failures = []
    for key, base in baseline_ix.items():
        if key not in current_ix:
            failures.append(f"Missing benchmark in current report: {key[0]} [{key[1]}]")
            continue

        curr = current_ix[key]
        for metric in ("cpu_insns", "mem_bytes"):
            base_val = float(base.get(metric, 0))
            curr_val = float(curr.get(metric, 0))
            if base_val <= 0:
                continue
            change_pct = ((curr_val - base_val) / base_val) * 100.0
            if change_pct > args.max_regression_pct:
                failures.append(
                    f"{key[0]} [{key[1]}] {metric} regression: {change_pct:.2f}% "
                    f"(baseline={base_val:.0f}, current={curr_val:.0f})"
                )

    if failures:
        print("Gas benchmark regressions detected:")
        for issue in failures:
            print(f" - {issue}")
        return 1

    print("Gas benchmarks are within configured regression budget.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
