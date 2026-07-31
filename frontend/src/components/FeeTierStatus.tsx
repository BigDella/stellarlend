import React from "react";

interface Tier {
  name: string;
  discountBps: number;
  loyaltyBonusBps: number;
}
interface FeeTierStatusProps {
  current: Tier;
  next?: Tier;
  progress?: Record<string, number> | null;
  totalSavings: number;
  effectiveAt: number;
}

export function FeeTierStatus({
  current,
  next,
  progress,
  totalSavings,
  effectiveAt,
}: FeeTierStatusProps) {
  return (
    <section aria-labelledby="tier-title" className="rounded-xl border p-5">
      <h2 id="tier-title" className="text-xl font-semibold">
        {current.name} fee tier
      </h2>
      <dl className="mt-3 grid grid-cols-2 gap-3">
        <div>
          <dt>Fee discount</dt>
          <dd>{(current.discountBps / 100).toFixed(0)}%</dd>
        </div>
        <div>
          <dt>Loyalty bonus</dt>
          <dd>{(current.loyaltyBonusBps / 100).toFixed(2)}%</dd>
        </div>
        <div>
          <dt>Total saved</dt>
          <dd>{totalSavings.toFixed(2)}</dd>
        </div>
        <div>
          <dt>Next evaluation</dt>
          <dd>{new Date(effectiveAt).toLocaleDateString()}</dd>
        </div>
      </dl>
      {next && progress && (
        <div className="mt-4">
          <h3 className="font-medium">Progress to {next.name}</h3>
          {Object.entries(progress).map(([metric, value]) => (
            <div key={metric} className="mt-2">
              <label className="capitalize">
                {metric}
                <progress className="ml-2" max={1} value={value}>
                  {Math.round(value * 100)}%
                </progress>
              </label>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
