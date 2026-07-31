#!/usr/bin/env node

import * as fs from 'fs';
import * as path from 'path';

interface GasData {
  [functionName: string]: {
    old: number;
    new: number;
    change: number;
    percentChange: number;
  };
}

interface CheckResult {
  total: number;
  improvements: number;
  regressions: number;
  average_change: number;
}

const args = process.argv.slice(2);
const baselineFile = args[0] || '/tmp/baseline.json';
const currentFile = args[1] || '/tmp/current.json';

function readGasData(file: string): Record<string, number> {
  if (!fs.existsSync(file)) {
    return {};
  }
  const data = fs.readFileSync(file, 'utf-8');
  return JSON.parse(data);
}

function compareGasData(
  baseline: Record<string, number>,
  current: Record<string, number>
): GasData {
  const comparison: GasData = {};
  const allFunctions = new Set([...Object.keys(baseline), ...Object.keys(current)]);

  for (const fn of allFunctions) {
    const oldGas = baseline[fn] || 0;
    const newGas = current[fn] || 0;
    const change = newGas - oldGas;
    const percentChange = oldGas > 0 ? (change / oldGas) * 100 : 0;

    comparison[fn] = {
      old: oldGas,
      new: newGas,
      change,
      percentChange,
    };
  }

  return comparison;
}

function generateMarkdownReport(comparison: GasData): string {
  const rows = Object.entries(comparison)
    .sort(([, a], [, b]) => Math.abs(b.percentChange) - Math.abs(a.percentChange))
    .map(([fn, data]) => {
      const changeStr = data.percentChange > 0 ? `+${data.percentChange.toFixed(1)}%` : `${data.percentChange.toFixed(1)}%`;
      const indicator = Math.abs(data.percentChange) > 5
        ? (data.percentChange > 0 ? '🔴' : '🟢')
        : '⚪';

      return `| ${fn} | ${data.old} | ${data.new} | ${data.change > 0 ? '+' : ''}${data.change} | ${changeStr} | ${indicator} |`;
    });

  return `| Function | Old Gas | New Gas | Change | % Change | Status |
|----------|---------|---------|--------|----------|--------|
${rows.join('\n')}`;
}

function checkRegressions(comparison: GasData): CheckResult {
  let improvements = 0;
  let regressions = 0;
  let totalChange = 0;
  let count = 0;

  for (const data of Object.values(comparison)) {
    if (data.new > 0 || data.old > 0) {
      if (data.percentChange > 5) {
        regressions++;
      } else if (data.percentChange < -5) {
        improvements++;
      }
      totalChange += data.percentChange;
      count++;
    }
  }

  return {
    total: count,
    improvements,
    regressions,
    average_change: count > 0 ? totalChange / count : 0,
  };
}

const baseline = readGasData(baselineFile);
const current = readGasData(currentFile);
const comparison = compareGasData(baseline, current);
const report = generateMarkdownReport(comparison);
const checkResult = checkRegressions(comparison);

if (args[2] === '--json') {
  console.log(JSON.stringify(checkResult, null, 2));
} else {
  console.log(report);
  console.log('\n### Check Result');
  console.log(JSON.stringify(checkResult, null, 2));
}

fs.writeFileSync('/tmp/check_result.json', JSON.stringify(checkResult));
