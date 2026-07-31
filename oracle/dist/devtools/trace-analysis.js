function collectFrames(invocation, depth, ancestry, frames, stateChanges) {
    const pathParts = [...ancestry, `${invocation.contractId}.${invocation.functionName}`];
    const path = pathParts.join(' > ');
    const localStateChanges = (invocation.stateChanges ?? []).map((change) => ({
        ...change,
        contractId: change.contractId ?? invocation.contractId,
    }));
    stateChanges.push(...localStateChanges);
    let cumulativeGasUsed = invocation.gasUsed;
    let maxDepth = depth;
    let totalInvocations = 1;
    for (const child of invocation.children ?? []) {
        const childResult = collectFrames(child, depth + 1, pathParts, frames, stateChanges);
        cumulativeGasUsed += childResult.cumulativeGasUsed;
        maxDepth = Math.max(maxDepth, childResult.maxDepth);
        totalInvocations += childResult.totalInvocations;
    }
    frames.push({
        depth,
        path,
        contractId: invocation.contractId,
        functionName: invocation.functionName,
        gasUsed: invocation.gasUsed,
        cumulativeGasUsed,
        cpuInstructions: invocation.cpuInstructions ?? 0,
        memoryBytes: invocation.memoryBytes ?? 0,
        stateChangeCount: localStateChanges.length,
    });
    return { cumulativeGasUsed, maxDepth, totalInvocations };
}
export function calculateTraceOverhead(baselineMs, tracingMs) {
    if (baselineMs === undefined ||
        tracingMs === undefined ||
        baselineMs <= 0 ||
        tracingMs < baselineMs) {
        return undefined;
    }
    const deltaMs = tracingMs - baselineMs;
    return {
        baselineMs,
        tracingMs,
        deltaMs,
        overheadPercent: Number(((deltaMs / baselineMs) * 100).toFixed(2)),
    };
}
export function analyzeTrace(snapshot, options = {}) {
    const hotPathLimit = options.hotPathLimit ?? 5;
    const largeTraceThreshold = options.largeTraceThreshold ?? 250;
    const callFrames = [];
    const stateChanges = [];
    let totalGasUsed = 0;
    let maxDepth = 0;
    let totalInvocations = 0;
    for (const invocation of snapshot.invocations) {
        const result = collectFrames(invocation, 0, [], callFrames, stateChanges);
        totalGasUsed += result.cumulativeGasUsed;
        maxDepth = Math.max(maxDepth, result.maxDepth);
        totalInvocations += result.totalInvocations;
    }
    callFrames.sort((left, right) => {
        if (left.depth !== right.depth) {
            return left.depth - right.depth;
        }
        return left.path.localeCompare(right.path);
    });
    const totalCpuInstructions = callFrames.reduce((sum, frame) => sum + frame.cpuInstructions, 0);
    const totalMemoryBytes = callFrames.reduce((sum, frame) => sum + frame.memoryBytes, 0);
    const hotPaths = [...callFrames]
        .sort((left, right) => right.cumulativeGasUsed - left.cumulativeGasUsed)
        .slice(0, hotPathLimit)
        .map((frame) => ({
        path: frame.path,
        gasUsed: frame.cumulativeGasUsed,
        percentageOfTotalGas: totalGasUsed === 0
            ? 0
            : Number(((frame.cumulativeGasUsed / totalGasUsed) * 100).toFixed(2)),
    }));
    const warnings = [];
    if (totalInvocations > largeTraceThreshold) {
        warnings.push(`Trace contains ${totalInvocations} invocations; consider filtering by contract or function before storing it long term.`);
    }
    if (stateChanges.length > 500) {
        warnings.push('Trace contains more than 500 state changes; archive the raw trace instead of logging it inline.');
    }
    if (callFrames.some((frame) => frame.gasUsed === 0 && frame.cumulativeGasUsed > 0)) {
        warnings.push('One or more frames reported zero local gas. Check the RPC simulator payload before using this trace for regression analysis.');
    }
    const traceOverhead = calculateTraceOverhead(snapshot.elapsedMs, snapshot.tracingElapsedMs);
    if (traceOverhead && traceOverhead.overheadPercent > 25) {
        warnings.push(`Trace overhead is ${traceOverhead.overheadPercent}%; disable tracing for load tests and keep it to focused debugging sessions.`);
    }
    return {
        transactionHash: snapshot.transactionHash,
        network: snapshot.network,
        ledger: snapshot.ledger,
        totalGasUsed,
        totalCpuInstructions,
        totalMemoryBytes,
        maxDepth,
        totalInvocations,
        totalStateChanges: stateChanges.length,
        callFrames,
        hotPaths,
        stateChanges,
        warnings,
        traceOverhead,
    };
}
//# sourceMappingURL=trace-analysis.js.map