/** @type {import('@stryker-mutator/api/core').PartialStrykerOptions} */
export default {
  // Mutate all TypeScript source files except test files and type definitions
  mutate: [
    'src/**/*.ts',
    '!src/**/*.test.ts',
    '!src/**/*.spec.ts',
    '!src/types/**/*.ts',
    '!src/index.ts',
    '!src/vercel.ts',
  ],
  
  testRunner: 'jest',
  coverageAnalysis: 'perTest',
  
  jest: {
    projectType: 'custom',
    configFile: 'jest.mutation.config.js',
    enableFindRelatedTests: true,
  },
  
  // Comprehensive reporting for analysis
  reporters: [
    'clear-text',
    'progress',
    'html',
    'json',
  ],
  
  htmlReporter: {
    fileName: 'reports/mutation/index.html',
  },
  
  jsonReporter: {
    fileName: 'reports/mutation/report.json',
  },
  
  // Dashboard reporter (optional - requires STRYKER_DASHBOARD_API_KEY)
  // Uncomment and configure if using Stryker Dashboard
  // dashboardReporter: {
  //   baseUrl: 'https://dashboard.stryker-mutator.io',
  //   reportType: 'full',
  //   projectName: 'stellarlend-api',
  //   version: process.env.GITHUB_SHA || 'local',
  //   module: 'api',
  // },
  
  tempDirName: '.stryker-tmp',
  
  // Performance optimization
  concurrency: 4,
  maxConcurrentTestRunners: 4,
  timeoutMS: 60000,
  timeoutFactor: 1.5,
  
  // Mutation score gate (>80%)
  thresholds: {
    high: 80,
    low: 75,
    break: 75,
  },
  
  // Ignore specific mutants that are equivalent or irrelevant
  ignorePatterns: [
    // Ignore type-only mutations
    'src/types/**/*.ts',
    // Ignore configuration files
    'src/config/**/*.ts',
    // Ignore middleware that may have side effects
    'src/middleware/**/*.ts',
  ],
  
  // Incremental mutation testing configuration
  incremental: true,
  incrementalFile: '.stryker-incremental',
  
  // Time budget for mutation testing
  timeBudget: {
    minutes: 30,
  },
  
  // Handle equivalent mutants
  ignoreMutations: [
    // Ignore mutations that are equivalent in TypeScript
    'String',
    'Boolean',
  ],
  
  // Performance optimization for large codebases
  disableBail: false,
  bail: true,
  
  // Enable TypeScript checking
  checkers: ['typescript'],
  
  typescriptChecker: {
    tsconfigFile: 'tsconfig.json',
  },
  
  // Plugin configuration
  plugins: [
    '@stryker-mutator/jest-runner',
    '@stryker-mutator/typescript-checker',
  ],
  
  // Log level for debugging
  logLevel: 'info',
  
  // Dry run for testing configuration
  dryRunOnly: process.env.DRY_RUN === 'true',
};