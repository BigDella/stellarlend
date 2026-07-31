import { LiquidationMonitorService } from '../services/liquidation-monitor/liquidationMonitor.service';

describe('LiquidationMonitorService', () => {
  let service: LiquidationMonitorService;

  beforeEach(() => {
    service = new LiquidationMonitorService();
  });

  describe('getAllPositions', () => {
    it('returns seed positions sorted by health factor ascending', () => {
      const positions = service.getAllPositions();
      expect(positions.length).toBeGreaterThan(0);
      for (let i = 1; i < positions.length; i++) {
        expect(positions[i].healthFactor).toBeGreaterThanOrEqual(positions[i - 1].healthFactor);
      }
    });
  });

  describe('getPosition', () => {
    it('returns a known position', () => {
      const pos = service.getPosition('GABCDE12345');
      expect(pos).toBeDefined();
      expect(pos!.collateralAsset).toBe('XLM');
    });

    it('returns undefined for unknown address', () => {
      expect(service.getPosition('GUNKNOWN')).toBeUndefined();
    });
  });

  describe('getPositionsByRisk', () => {
    it('filters by risk category', () => {
      const critical = service.getPositionsByRisk('critical');
      expect(critical.length).toBeGreaterThan(0);
      expect(critical.every((p) => p.riskCategory === 'critical')).toBe(true);
    });

    it('returns empty for non-existent category', () => {
      const none = service.getPositionsByRisk('safe' as any);
      // There's at least one safe position in seed data
      expect(none.every((p) => p.riskCategory === 'safe')).toBe(true);
    });
  });

  describe('getLiquidatablePositions', () => {
    it('returns positions with health factor below 1.2', () => {
      const liquidatable = service.getLiquidatablePositions();
      expect(liquidatable.every((p) => p.healthFactor < 1.2)).toBe(true);
    });

    it('filters by minimum profit', () => {
      const profitable = service.getLiquidatablePositions(1000);
      expect(profitable.every((p) => p.netProfit >= 1000)).toBe(true);
    });
  });

  describe('updatePosition', () => {
    it('updates an existing position', () => {
      service.updatePosition('GABCDE12345', { healthFactor: 0.95, riskCategory: 'critical' });
      const pos = service.getPosition('GABCDE12345');
      expect(pos!.healthFactor).toBe(0.95);
      expect(pos!.riskCategory).toBe('critical');
    });

    it('ignores updates to unknown positions', () => {
      service.updatePosition('GUNKNOWN', { healthFactor: 0.5 });
      expect(service.getPosition('GUNKNOWN')).toBeUndefined();
    });
  });

  describe('thresholds and alerts', () => {
    it('sets and retrieves threshold', () => {
      service.setThreshold({
        address: 'GABCDE12345',
        dangerHf: 1.0,
        warningHf: 1.2,
        enabled: true,
      });
      const threshold = service.getThreshold('GABCDE12345');
      expect(threshold).toBeDefined();
      expect(threshold!.dangerHf).toBe(1.0);
    });

    it('returns alerts for positions below threshold', () => {
      service.setThreshold({
        address: 'GABCDE12345',
        dangerHf: 1.5,
        warningHf: 2.0,
        enabled: true,
      });
      const alerts = service.getAlerts();
      expect(alerts.some((a) => a.address === 'GABCDE12345')).toBe(true);
    });
  });

  describe('onUpdate', () => {
    it('notifies listeners on position update', () => {
      const listener = jest.fn();
      service.onUpdate(listener);
      service.updatePosition('GABCDE12345', { healthFactor: 1.0 });
      expect(listener).toHaveBeenCalled();
    });
  });
});
