import type { LiquidationEvent, LiquidationMetrics } from '../types.js';

/**
 * Extract liquidation events from archived/raw event payloads.
 */
export function extractLiquidations(
  events: Array<{
    topic?: string;
    eventName?: string;
    txHash: string;
    ledger: number;
    timestamp: Date | string;
    payload: Record<string, unknown>;
  }>
): LiquidationEvent[] {
  return events
    .filter((e) => (e.topic || e.eventName) === 'liquidation_event')
    .map((e) => {
      const payload = e.payload;
      return {
        ledger: e.ledger,
        txHash: e.txHash,
        timestamp: e.timestamp instanceof Date ? e.timestamp : new Date(e.timestamp),
        liquidator: String(payload.liquidator ?? ''),
        borrower: String(payload.borrower ?? ''),
        debtAsset: payload.debt_asset == null ? null : String(payload.debt_asset),
        collateralAsset:
          payload.collateral_asset == null ? null : String(payload.collateral_asset),
        debtLiquidated: Number(payload.debt_liquidated ?? 0),
        collateralSeized: Number(payload.collateral_seized ?? 0),
        incentiveAmount: Number(payload.incentive_amount ?? 0),
        debtAssetPrice: payload.debt_asset_price != null ? Number(payload.debt_asset_price) : 1,
        collateralAssetPrice:
          payload.collateral_asset_price != null ? Number(payload.collateral_asset_price) : 1,
        gasCost: payload.gas_cost != null ? Number(payload.gas_cost) : 0,
      };
    });
}

/**
 * Compute per-liquidation analytics metrics.
 *
 * discount ≈ (collateralValue - debtValue) / debtValue
 * profit ≈ collateralValue - debtValue - gasCost (incentive counted in collateral side)
 */
export function computeLiquidationMetrics(event: LiquidationEvent): LiquidationMetrics {
  const debtPrice = event.debtAssetPrice ?? 1;
  const collPrice = event.collateralAssetPrice ?? 1;
  const debtValue = event.debtLiquidated * debtPrice;
  const collateralValue = event.collateralSeized * collPrice;
  const gasCost = event.gasCost ?? 0;
  const discount = debtValue === 0 ? 0 : (collateralValue - debtValue) / debtValue;
  const profit = collateralValue - debtValue + event.incentiveAmount * collPrice;
  const netProfit = profit - gasCost;
  const ts = event.timestamp;

  return {
    txHash: event.txHash,
    timestamp: ts,
    discount,
    profit,
    gasCost,
    netProfit,
    hourOfDay: ts.getUTCHours(),
    dayOfWeek: ts.getUTCDay(),
    collateralAsset: event.collateralAsset ?? 'native',
    debtAsset: event.debtAsset ?? 'native',
    debtLiquidated: event.debtLiquidated,
    collateralSeized: event.collateralSeized,
  };
}
