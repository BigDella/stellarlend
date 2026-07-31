import React, { useState } from "react";

interface Pool {
  poolId: string;
  apyBps: number;
  riskGrade: string;
  capacity: number;
  weightBps: number;
}
interface BudgetPlannerProps {
  pools: Pool[];
  apiBaseUrl?: string;
}
interface PlanStep {
  order: number;
  poolId: string;
  amount: number;
}
interface BudgetPlan {
  projectedBalance: number;
  totalProjectedReturn: number;
  goalOnTrack: boolean;
  steps: PlanStep[];
}

export function BudgetPlanner({
  pools: initialPools,
  apiBaseUrl = "/api",
}: BudgetPlannerProps) {
  const [capital, setCapital] = useState(10_000);
  const [pools, setPools] = useState(initialPools);
  const [plan, setPlan] = useState<BudgetPlan | null>(null);
  function updateWeight(poolId: string, weightBps: number) {
    setPools((current) =>
      current.map((pool) =>
        pool.poolId === poolId ? { ...pool, weightBps } : pool,
      ),
    );
  }
  async function project() {
    const response = await fetch(`${apiBaseUrl}/planner/budget`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        capital,
        horizonDays: 365,
        rebalanceThresholdBps: 100,
        maxRiskExposureBps: Object.fromEntries(
          pools.map((p) => [p.riskGrade, 10_000]),
        ),
        pools,
      }),
    });
    const result = (await response.json()) as { data?: BudgetPlan };
    setPlan(result.data ?? null);
  }
  return (
    <section aria-labelledby="planner-title" className="space-y-4">
      <h2 id="planner-title" className="text-xl font-semibold">
        Lending budget planner
      </h2>
      <label className="block">
        Capital
        <input
          type="number"
          min={1}
          value={capital}
          onChange={(event) => setCapital(Number(event.target.value))}
          className="ml-2 rounded border p-2"
        />
      </label>
      {pools.map((pool) => (
        <label key={pool.poolId} className="block">
          {pool.poolId} ({pool.riskGrade}){" "}
          <input
            aria-label={`${pool.poolId} allocation`}
            type="range"
            min={0}
            max={10_000}
            value={pool.weightBps}
            onChange={(event) =>
              updateWeight(pool.poolId, Number(event.target.value))
            }
          />{" "}
          {(pool.weightBps / 100).toFixed(0)}%
        </label>
      ))}
      <button
        type="button"
        onClick={() => void project()}
        className="rounded bg-blue-600 px-4 py-2 text-white"
      >
        Build plan
      </button>
      {plan && (
        <div aria-live="polite" className="rounded-xl border p-4">
          <p>Projected balance: {plan.projectedBalance.toFixed(2)}</p>
          <p>Projected return: {plan.totalProjectedReturn.toFixed(2)}</p>
          <p>{plan.goalOnTrack ? "Goal on track" : "Goal needs adjustment"}</p>
          <ol className="list-decimal pl-5">
            {plan.steps.map((step) => (
              <li key={`${step.poolId}-${step.order}`}>
                Deposit {step.amount} into {step.poolId}
              </li>
            ))}
          </ol>
        </div>
      )}
    </section>
  );
}
