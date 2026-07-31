import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { analyzeTrace } from './trace-analysis.js';
async function main() {
    const traceFile = process.argv[2];
    if (!traceFile) {
        console.error('Usage: npm run trace:analyze -- <trace-file.json>');
        process.exitCode = 1;
        return;
    }
    const rawTrace = await readFile(resolve(traceFile), 'utf8');
    const snapshot = JSON.parse(rawTrace);
    const analysis = analyzeTrace(snapshot);
    console.log(JSON.stringify(analysis, null, 2));
}
main().catch((error) => {
    console.error('Failed to analyze trace', error);
    process.exitCode = 1;
});
//# sourceMappingURL=trace-report.js.map