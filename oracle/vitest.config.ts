import { defineConfig } from 'vitest/config';
import path from 'path';

export default defineConfig({
    test: {
        globals: true,
        environment: 'node',
        include: ['tests/**/*.test.ts'],
        coverage: {
            provider: 'v8',
            reporter: ['text', 'html', 'lcov'],
            include: ['src/**/*.ts'],
            exclude: ['src/index.ts'],
            thresholds: {
                lines: 80,
                functions: 85,
                branches: 85,
                statements: 80,
            },
        },
    },
    resolve: {
        alias: {
            '@/claims': path.resolve(__dirname, './src/claims'),
            '@/config': path.resolve(__dirname, './src/config'),
            '@/devtools': path.resolve(__dirname, './src/devtools'),
            '@/providers': path.resolve(__dirname, './src/providers'),
            '@/security': path.resolve(__dirname, './src/security'),
            '@/services': path.resolve(__dirname, './src/services'),
            '@/types': path.resolve(__dirname, './src/types'),
            '@/utils': path.resolve(__dirname, './src/utils'),
        },
    },
});
