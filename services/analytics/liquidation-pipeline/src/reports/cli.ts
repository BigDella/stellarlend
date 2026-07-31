import { promises as fs } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { pipelineFromRawEvents, writeReportFiles } from './generate.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

async function main(): Promise<void> {
  const period = (process.argv[2] as 'daily' | 'weekly' | 'monthly') || 'daily';
  const inputPath = process.argv[3];
  const now = new Date();
  const from = new Date(now);
  if (period === 'daily') from.setUTCDate(from.getUTCDate() - 1);
  if (period === 'weekly') from.setUTCDate(from.getUTCDate() - 7);
  if (period === 'monthly') from.setUTCDate(from.getUTCDate() - 30);

  let raw: Array<{
    topic?: string;
    eventName?: string;
    txHash: string;
    ledger: number;
    timestamp: string;
    payload: Record<string, unknown>;
  }> = [];

  if (inputPath) {
    raw = JSON.parse(await fs.readFile(inputPath, 'utf8')) as typeof raw;
  }

  const report = pipelineFromRawEvents(raw, period, from, now);
  const reportsDir = path.resolve(__dirname, '../../reports');
  const paths = await writeReportFiles(report, reportsDir);
  console.log(JSON.stringify({ report: paths, total: report.totalLiquidations }, null, 2));
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
