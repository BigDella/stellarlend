#!/usr/bin/env node

/**
 * Survived Mutant Analysis Script
 * 
 * This script analyzes survived mutants from Stryker mutation testing
 * and provides recommendations for test improvements.
 * 
 * Usage: node scripts/analyze-mutants.js <path-to-mutation-report>
 */

const fs = require('fs');
const path = require('path');

class MutantAnalyzer {
  constructor(reportPath) {
    this.reportPath = reportPath;
    this.report = null;
    this.survivedMutants = [];
    this.killedMutants = [];
    this.timedOutMutants = [];
    this.equivalentMutants = [];
  }

  loadReport() {
    try {
      const reportContent = fs.readFileSync(this.reportPath, 'utf8');
      this.report = JSON.parse(reportContent);
      console.log(`✓ Loaded mutation report from ${this.reportPath}`);
      return true;
    } catch (error) {
      console.error(`✗ Failed to load mutation report: ${error.message}`);
      return false;
    }
  }

  categorizeMutants() {
    if (!this.report || !this.report.mutationScores) {
      console.error('✗ Invalid mutation report format');
      return false;
    }

    const { mutationScores } = this.report;
    
    mutationScores.forEach(file => {
      file.mutants.forEach(mutant => {
        switch (mutant.status) {
          case 'Survived':
            this.survivedMutants.push({ file: file.name, mutant });
            break;
          case 'Killed':
            this.killedMutants.push({ file: file.name, mutant });
            break;
          case 'TimedOut':
            this.timedOutMutants.push({ file: file.name, mutant });
            break;
          case 'NoCoverage':
            this.timedOutMutants.push({ file: file.name, mutant });
            break;
          case 'Ignored':
            // Ignore ignored mutants
            break;
          default:
            console.warn(`Unknown mutant status: ${mutant.status}`);
        }
      });
    });

    console.log(`\nMutant Analysis Summary:`);
    console.log(`  Survived: ${this.survivedMutants.length}`);
    console.log(`  Killed: ${this.killedMutants.length}`);
    console.log(`  TimedOut/NoCoverage: ${this.timedOutMutants.length}`);
    
    return true;
  }

  analyzeSurvivedMutants() {
    console.log('\n=== Survived Mutant Analysis ===\n');
    
    const recommendations = [];
    
    this.survivedMutants.forEach(({ file, mutant }) => {
      const recommendation = this.generateRecommendation(file, mutant);
      if (recommendation) {
        recommendations.push(recommendation);
      }
    });

    if (recommendations.length === 0) {
      console.log('✓ No survived mutants requiring action');
      return [];
    }

    console.log(`Found ${recommendations.length} survived mutants requiring attention:\n`);
    
    recommendations.forEach((rec, index) => {
      console.log(`${index + 1}. ${rec.title}`);
      console.log(`   File: ${rec.file}`);
      console.log(`   Line: ${rec.line}`);
      console.log(`   Mutation: ${rec.mutation}`);
      console.log(`   Recommendation: ${rec.recommendation}\n`);
    });

    return recommendations;
  }

  generateRecommendation(file, mutant) {
    const { mutatorName, replacement, location, description } = mutant;
    
    // Generate specific recommendations based on mutator type
    const recommendations = {
      'ArithmeticOperator': {
        title: 'Arithmetic Operator Mutation Survived',
        recommendation: 'Add test case that verifies the exact arithmetic operation. Test both positive and negative edge cases.',
      },
      'EqualityOperator': {
        title: 'Equality Operator Mutation Survived',
        recommendation: 'Add test case that specifically tests the equality condition. Consider testing boundary values.',
      },
      'LogicalOperator': {
        title: 'Logical Operator Mutation Survived',
        recommendation: 'Add test case that exercises both branches of the logical condition.',
      },
      'ConditionalExpression': {
        title: 'Conditional Expression Mutation Survived',
        recommendation: 'Add test case that covers the alternative branch of the condition.',
      },
      'BlockStatement': {
        title: 'Block Statement Mutation Survived',
        recommendation: 'Add test case that verifies the code block is executed. Consider testing side effects.',
      },
      'StringLiteral': {
        title: 'String Literal Mutation Survived',
        recommendation: 'Add test case that verifies the exact string value or behavior dependent on this string.',
      },
      'BooleanLiteral': {
        title: 'Boolean Literal Mutation Survived',
        recommendation: 'Add test case that tests both true and false scenarios for this boolean value.',
      },
      'ArrayLiteral': {
        title: 'Array Literal Mutation Survived',
        recommendation: 'Add test case that verifies array contents and handles empty array scenarios.',
      },
      'ObjectLiteral': {
        title: 'Object Literal Mutation Survived',
        recommendation: 'Add test case that verifies object structure and handles missing properties.',
      },
      'UnaryOperator': {
        title: 'Unary Operator Mutation Survived',
        recommendation: 'Add test case that tests the negated/positive version of the value.',
      },
      'UpdateOperator': {
        title: 'Update Operator Mutation Survived',
        recommendation: 'Add test case that verifies the increment/decrement behavior and boundary conditions.',
      },
      'FunctionDeclaration': {
        title: 'Function Declaration Mutation Survived',
        recommendation: 'Add test case that specifically tests this function with various inputs.',
      },
      'ReturnStatement': {
        title: 'Return Statement Mutation Survived',
        recommendation: 'Add test case that verifies the return value and handles undefined/null cases.',
      },
    };

    const rec = recommendations[mutatorName] || {
      title: 'Unknown Mutation Type',
      recommendation: 'Review the mutation and add appropriate test coverage.',
    };

    return {
      title: rec.title,
      file,
      line: location.start.line,
      mutation: `${mutatorName}: ${replacement}`,
      recommendation: rec.recommendation,
      description,
    };
  }

  generateTestImprovementPlan(recommendations) {
    if (recommendations.length === 0) {
      console.log('\n✓ No test improvements needed');
      return;
    }

    console.log('\n=== Test Improvement Plan ===\n');
    
    // Group recommendations by file
    const byFile = {};
    recommendations.forEach(rec => {
      if (!byFile[rec.file]) {
        byFile[rec.file] = [];
      }
      byFile[rec.file].push(rec);
    });

    Object.keys(byFile).forEach(file => {
      console.log(`File: ${file}`);
      console.log(`  Survived Mutants: ${byFile[file].length}`);
      console.log('  Actions:');
      byFile[file].forEach(rec => {
        console.log(`    - Line ${rec.line}: ${rec.recommendation}`);
      });
      console.log('');
    });

    // Generate priority list
    console.log('Priority Order (most critical first):');
    console.log('1. Arithmetic and logical mutations (core logic)');
    console.log('2. Equality and conditional mutations (control flow)');
    console.log('3. String and boolean mutations (data validation)');
    console.log('4. Object and array mutations (data structures)');
    console.log('');
  }

  calculateMutationScore() {
    if (!this.report || !this.report.mutationScores) {
      return null;
    }

    const total = this.survivedMutants.length + this.killedMutants.length + this.timedOutMutants.length;
    if (total === 0) {
      return 100;
    }

    const score = (this.killedMutants.length / total) * 100;
    return Math.round(score * 100) / 100;
  }

  generateSummary() {
    const score = this.calculateMutationScore();
    const total = this.survivedMutants.length + this.killedMutants.length + this.timedOutMutants.length;
    
    console.log('\n=== Mutation Score Summary ===\n');
    console.log(`Mutation Score: ${score}%`);
    console.log(`Total Mutants: ${total}`);
    console.log(`Killed: ${this.killedMutants.length} (${Math.round((this.killedMutants.length / total) * 100)}%)`);
    console.log(`Survived: ${this.survivedMutants.length} (${Math.round((this.survivedMutants.length / total) * 100)}%)`);
    console.log(`TimedOut/NoCoverage: ${this.timedOutMutants.length} (${Math.round((this.timedOutMutants.length / total) * 100)}%)`);
    
    if (score < 80) {
      console.log('\n⚠ Mutation score below 80% threshold. Test improvements required.');
    } else {
      console.log('\n✓ Mutation score meets 80% threshold.');
    }
  }

  run() {
    console.log('=== Survived Mutant Analysis ===\n');
    
    if (!this.loadReport()) {
      process.exit(1);
    }

    if (!this.categorizeMutants()) {
      process.exit(1);
    }

    const recommendations = this.analyzeSurvivedMutants();
    this.generateTestImprovementPlan(recommendations);
    this.generateSummary();

    // Exit with error code if score is below threshold
    const score = this.calculateMutationScore();
    if (score < 80) {
      console.log('\n✗ Mutation score below 80% threshold');
      process.exit(1);
    } else {
      console.log('\n✓ Mutation score meets threshold');
      process.exit(0);
    }
  }
}

// Main execution
const reportPath = process.argv[2];
if (!reportPath) {
  console.error('Usage: node scripts/analyze-mutants.js <path-to-mutation-report>');
  process.exit(1);
}

const analyzer = new MutantAnalyzer(reportPath);
analyzer.run();
