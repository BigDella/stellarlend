import React, { useState } from "react";

interface Policy {
  id: string;
  providerId: string;
  coverageAmount: number;
  premiumBps: number;
  durationDays: number;
  terms: string;
  coveredTriggers: string[];
  exclusions: string[];
}
interface InsuranceMarketplaceProps {
  policies: Policy[];
  lender: string;
  positionId: string;
  apiBaseUrl?: string;
}

interface ClaimSummary {
  submitted: number;
  approved: number;
  denied: number;
  disputed: number;
}

export function InsuranceClaimsDashboard({ totals }: { totals: ClaimSummary }) {
  return (
    <section aria-labelledby="claims-title" className="rounded-xl border p-4">
      <h2 id="claims-title" className="text-xl font-semibold">
        Claims dashboard
      </h2>
      <dl className="mt-3 grid grid-cols-2 gap-3 sm:grid-cols-4">
        {Object.entries(totals).map(([status, count]) => (
          <div key={status} className="rounded-lg bg-slate-50 p-3">
            <dt className="capitalize text-slate-600">{status}</dt>
            <dd className="text-2xl font-semibold">{count}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}

export function InsuranceMarketplace({
  policies,
  lender,
  positionId,
  apiBaseUrl = "/api",
}: InsuranceMarketplaceProps) {
  const [message, setMessage] = useState("");
  async function purchase(policy: Policy) {
    setMessage("Purchasing coverage…");
    const response = await fetch(`${apiBaseUrl}/insurance/coverages`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        policyId: policy.id,
        lender,
        positionId,
        coverageAmount: policy.coverageAmount,
      }),
    });
    const result = await response.json();
    setMessage(
      response.ok
        ? `Coverage active. Premium: ${(policy.coverageAmount * policy.premiumBps) / 10_000}`
        : (result.error ?? "Purchase failed"),
    );
  }
  return (
    <section aria-labelledby="insurance-title" className="space-y-4">
      <h2 id="insurance-title" className="text-xl font-semibold">
        Deposit insurance
      </h2>
      <p aria-live="polite" className="text-sm text-slate-600">
        {message}
      </p>
      <div className="grid gap-4 md:grid-cols-2">
        {policies.map((policy) => (
          <article key={policy.id} className="rounded-xl border p-4">
            <h3 className="font-semibold">
              Coverage {policy.coverageAmount.toLocaleString()}
            </h3>
            <p>
              {policy.durationDays} days ·{" "}
              {(policy.premiumBps / 100).toFixed(2)}% premium
            </p>
            <p className="mt-2 text-sm">
              Covered: {policy.coveredTriggers.join(", ")}
            </p>
            <p className="text-sm text-slate-500">
              Excluded: {policy.exclusions.join(", ") || "None"}
            </p>
            <details className="my-3 text-sm">
              <summary>Policy terms</summary>
              {policy.terms}
            </details>
            <button
              type="button"
              onClick={() => void purchase(policy)}
              className="rounded bg-blue-600 px-4 py-2 text-white"
            >
              Protect position
            </button>
          </article>
        ))}
      </div>
    </section>
  );
}
