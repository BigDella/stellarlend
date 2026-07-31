export interface ContractStateChange {
    contractId?: string;
    key: string;
    operation: 'create' | 'update' | 'delete' | 'extend';
    before?: string | null;
    after?: string | null;
}
export interface ContractInvocationTrace {
    contractId: string;
    functionName: string;
    gasUsed: number;
    cpuInstructions?: number;
    memoryBytes?: number;
    events?: string[];
    stateChanges?: ContractStateChange[];
    children?: ContractInvocationTrace[];
}
export interface ContractTraceSnapshot {
    transactionHash?: string;
    network?: string;
    ledger?: number;
    elapsedMs?: number;
    tracingElapsedMs?: number;
    invocations: ContractInvocationTrace[];
}
export interface TraceCallFrame {
    depth: number;
    path: string;
    contractId: string;
    functionName: string;
    gasUsed: number;
    cumulativeGasUsed: number;
    cpuInstructions: number;
    memoryBytes: number;
    stateChangeCount: number;
}
export interface TraceHotPath {
    path: string;
    gasUsed: number;
    percentageOfTotalGas: number;
}
export interface TraceOverhead {
    baselineMs: number;
    tracingMs: number;
    deltaMs: number;
    overheadPercent: number;
}
export interface TraceAnalysis {
    transactionHash?: string;
    network?: string;
    ledger?: number;
    totalGasUsed: number;
    totalCpuInstructions: number;
    totalMemoryBytes: number;
    maxDepth: number;
    totalInvocations: number;
    totalStateChanges: number;
    callFrames: TraceCallFrame[];
    hotPaths: TraceHotPath[];
    stateChanges: ContractStateChange[];
    warnings: string[];
    traceOverhead?: TraceOverhead;
}
interface AnalyzeTraceOptions {
    hotPathLimit?: number;
    largeTraceThreshold?: number;
}
export declare function calculateTraceOverhead(baselineMs: number | undefined, tracingMs: number | undefined): TraceOverhead | undefined;
export declare function analyzeTrace(snapshot: ContractTraceSnapshot, options?: AnalyzeTraceOptions): TraceAnalysis;
export {};
//# sourceMappingURL=trace-analysis.d.ts.map