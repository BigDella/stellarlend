#!/usr/bin/env node

/**
 * Historical Mutation Score Tracking Script
 * 
 * This script tracks mutation scores over time and stores them in a JSON file
 * for historical analysis and trend monitoring.
 * 
 * Usage: node scripts/track-mutation-score.js <module> <path-to-mutation-report>
 */

const fs = require('fs');
const path = require('path');

class MutationScoreTracker {
  constructor(module, reportPath) {
    this.module = module;
    this.reportPath = reportPath;
    this.historyPath = path.join(process.cwd(), '.mutation-history.json');
    this.history = this.loadHistory();
  }

  loadHistory() {
    try {
      if (fs.existsSync(this.historyPath)) {
        const content = fs.readFileSync(this.historyPath, 'utf8');
        return JSON.parse(content);
      }
    } catch (error) {
      console.warn('Failed to load history file, starting fresh:', error.message);
    }
    
    return {
      api: [],
      oracle: [],
    };
  }

  saveHistory() {
    try {
      fs.writeFileSync(this.historyPath, JSON.stringify(this.history, null, 2));
      console.log(`✓ Saved mutation history to ${this.historyPath}`);
      return true;
    } catch (error) {
      console.error(`✗ Failed to save history: ${error.message}`);
      return false;
    }
  }

  loadReport() {
    try {
      const reportContent = fs.readFileSync(this.reportPath, 'utf8');
      return JSON.parse(reportContent);
    } catch (error) {
      console.error(`✗ Failed to load mutation report: ${error.message}`);
      return null;
    }
  }

  calculateScore(report) {
    if (!report || !report.mutationScores) {
      return null;
    }

    let totalKilled = 0;
    let totalSurvived = 0;
    let totalTimedOut = 0;

    report.mutationScores.forEach(file => {
      file.mutants.forEach(mutant => {
        if (mutant.status === 'Killed') totalKilled++;
        else if (mutant.status === 'Survived') totalSurvived++;
        else totalTimedOut++;
      });
    });

    const total = totalKilled + totalSurvived + totalTimedOut;
    const score = total > 0 ? Math.round((totalKilled / total) * 100) : 100;

    return {
      score,
      killed: totalKilled,
      survived: totalSurvived,
      timedOut: totalTimedOut,
      total,
    };
  }

  recordScore(scoreData) {
    const entry = {
      timestamp: new Date().toISOString(),
      gitSha: process.env.GITHUB_SHA || 'local',
      gitRef: process.env.GITHUB_REF || 'local',
      runNumber: process.env.GITHUB_RUN_NUMBER || 'local',
      ...scoreData,
    };

    this.history[this.module].push(entry);

    // Keep only the last 100 entries to prevent file from growing too large
    if (this.history[this.module].length > 100) {
      this.history[this.module] = this.history[this.module].slice(-100);
    }

    console.log(`✓ Recorded mutation score for ${this.module}: ${scoreData.score}%`);
  }

  analyzeTrends() {
    const scores = this.history[this.module];
    if (scores.length < 2) {
      console.log(`\nNot enough data points for trend analysis (need at least 2)`);
      return;
    }

    const recentScores = scores.slice(-10); // Last 10 runs
    const avgScore = recentScores.reduce((sum, entry) => sum + entry.score, 0) / recentScores.length;
    
    const latest = scores[scores.length - 1];
    const previous = scores[scores.length - 2];
    const change = latest.score - previous.score;

    console.log(`\n=== Mutation Score Trend Analysis (${this.module}) ===\n`);
    console.log(`Latest Score: ${latest.score}%`);
    console.log(`Previous Score: ${previous.score}%`);
    console.log(`Change: ${change > 0 ? '+' : ''}${change}%`);
    console.log(`Average (last 10): ${Math.round(avgScore)}%`);
    console.log(`Total Runs: ${scores.length}`);

    // Detect significant drops
    if (change < -5) {
      console.log(`⚠️ WARNING: Significant score drop detected (${change}%)`);
    } else if (change > 5) {
      console.log(`✅ Significant score improvement detected (+${change}%)`);
    }

    // Check if below threshold
    if (latest.score < 80) {
      console.log(`⚠️ Latest score below 80% threshold`);
    }
  }

  generateReport() {
    const scores = this.history[this.module];
    if (scores.length === 0) {
      console.log('No historical data available');
      return;
    }

    console.log(`\n=== Historical Mutation Scores (${this.module}) ===\n`);
    
    scores.slice(-10).reverse().forEach((entry, index) => {
      const date = new Date(entry.timestamp).toLocaleDateString();
      const status = entry.score >= 80 ? '✅' : '❌';
      console.log(`${status} ${date} - Score: ${entry.score}% (Killed: ${entry.killed}, Survived: ${entry.survived}, Total: ${entry.total})`);
    });
  }

  run() {
    console.log(`=== Historical Mutation Score Tracking (${this.module}) ===\n`);

    const report = this.loadReport();
    if (!report) {
      process.exit(1);
    }

    const scoreData = this.calculateScore(report);
    if (!scoreData) {
      console.error('✗ Failed to calculate mutation score');
      process.exit(1);
    }

    this.recordScore(scoreData);
    this.saveHistory();
    this.analyzeTrends();
    this.generateReport();

    process.exit(0);
  }
}

// Main execution
const module = process.argv[2];
const reportPath = process.argv[3];

if (!module || !reportPath) {
  console.error('Usage: node scripts/track-mutation-score.js <module> <path-to-mutation-report>');
  console.error('Example: node scripts/track-mutation-score.js api reports/mutation/report.json');
  process.exit(1);
}

const tracker = new MutationScoreTracker(module, reportPath);
tracker.run();
