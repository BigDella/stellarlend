import React, { useState } from 'react';

interface Position {
  collateral: number;
  debt: number;
  asset: string;
}

interface SimulationResult {
  scenarioName: string;
  currentHealth: number;
  afterScenarioHealth: number;
  isLiquidatable: boolean;
  liquidationPrice: number;
  safetyMargin: number;
  collateralChange?: number;
  debtChange?: number;
}

export const PositionSimulator: React.FC = () => {
  const [position, setPosition] = useState<Position>({
    collateral: 1000,
    debt: 500,
    asset: 'USDC',
  });

  const [simulationResults, setSimulationResults] = useState<SimulationResult[]>([]);
  const [selectedForComparison, setSelectedForComparison] = useState<number[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<'scenarios' | 'complex' | 'history' | 'compare'>('scenarios');
  const [complexParams, setComplexParams] = useState({ priceDrop: 10, rateIncrease: 5 });
  const [historyDate, setHistoryDate] = useState('2021-05-19');
  const [realTimeAmount, setRealTimeAmount] = useState(0);
  const [realTimeType, setRealTimeType] = useState<'deposit' | 'withdraw' | 'borrow' | 'repay'>('deposit');
  const [realTimeResult, setRealTimeResult] = useState<SimulationResult | null>(null);

  const handleSimulate = async (endpoint: string, body: Record<string, unknown>) => {
    setIsLoading(true);
    try {
      const response = await fetch(`/api/simulator/${endpoint}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      const data = await response.json();
      if (data.success) {
        const result: SimulationResult = {
          scenarioName: body.scenarioName as string || endpoint,
          currentHealth: data.data.current || data.data.initial_health,
          afterScenarioHealth: data.data.after_scenario || data.data.final_health,
          isLiquidatable: data.data.is_liquidatable,
          liquidationPrice: data.data.liquidation_price || 0,
          safetyMargin: data.data.safety_margin || 0,
          collateralChange: data.data.collateral_change,
          debtChange: data.data.debt_change,
        };
        setSimulationResults(prev => [...prev, result]);
      }
    } catch (error) {
      console.error('Simulation failed:', error);
    }
    setIsLoading(false);
  };

  const handleRealTimeSimulate = async () => {
    try {
      const response = await fetch('/api/simulator/realtime', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          position,
          changeType: realTimeType,
          amount: realTimeAmount,
        }),
      });
      const data = await response.json();
      if (data.success) {
        setRealTimeResult({
          scenarioName: `Real-time: ${realTimeType} ${realTimeAmount}`,
          currentHealth: data.data.current,
          afterScenarioHealth: data.data.after_scenario,
          isLiquidatable: data.data.is_liquidatable,
          liquidationPrice: data.data.liquidation_price,
          safetyMargin: data.data.safety_margin,
        });
      }
    } catch (error) {
      console.error('Real-time simulation failed:', error);
    }
  };

  const handleCompare = async () => {
    if (selectedForComparison.length !== 2) return;
    const scenarioA = simulationResults[selectedForComparison[0]];
    const scenarioB = simulationResults[selectedForComparison[1]];
    try {
      const response = await fetch('/api/simulator/compare', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ scenarioA, scenarioB }),
      });
      const data = await response.json();
      if (data.success) {
        const comparison: SimulationResult = {
          scenarioName: `Comparison: ${scenarioA.scenarioName} vs ${scenarioB.scenarioName}`,
          currentHealth: scenarioA.afterScenarioHealth,
          afterScenarioHealth: data.data.health_difference,
          isLiquidatable: false,
          liquidationPrice: 0,
          safetyMargin: 0,
        };
        setSimulationResults(prev => [...prev, comparison]);
        setSelectedForComparison([]);
        setActiveTab('scenarios');
      }
    } catch (error) {
      console.error('Comparison failed:', error);
    }
  };

  const getHealthColor = (health: number): string => {
    if (health >= 2) return '#28a745';
    if (health >= 1.5) return '#ffc107';
    if (health >= 1) return '#fd7e14';
    return '#dc3545';
  };

  const currentHealth = position.debt > 0 ? position.collateral / position.debt : Infinity;

  return (
    <div style={styles.container}>
      <h2>Position Health Simulator</h2>

      <div style={styles.positionCard}>
        <h3>Current Position</h3>
        <div style={styles.positionDetails}>
          <div>
            <label style={styles.fieldLabel}>Collateral</label>
            <input
              type="number"
              value={position.collateral}
              onChange={e => setPosition(prev => ({ ...prev, collateral: parseFloat(e.target.value) || 0 }))}
              style={styles.input}
            />
          </div>
          <div>
            <label style={styles.fieldLabel}>Debt</label>
            <input
              type="number"
              value={position.debt}
              onChange={e => setPosition(prev => ({ ...prev, debt: parseFloat(e.target.value) || 0 }))}
              style={styles.input}
            />
          </div>
          <div>
            <label style={styles.fieldLabel}>Health Factor</label>
            <div
              style={{
                ...styles.healthFactor,
                backgroundColor: getHealthColor(currentHealth),
              }}
            >
              {currentHealth === Infinity ? 'No Debt' : currentHealth.toFixed(2)}
            </div>
          </div>
        </div>
      </div>

      <div style={styles.tabBar}>
        {(['scenarios', 'complex', 'history', 'compare'] as const).map(tab => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            style={{
              ...styles.tab,
              backgroundColor: activeTab === tab ? '#007bff' : '#f0f0f0',
              color: activeTab === tab ? 'white' : '#333',
            }}
          >
            {tab === 'scenarios' ? 'Basic Scenarios' : tab === 'complex' ? 'Complex' : tab === 'history' ? 'Historical' : 'Compare'}
          </button>
        ))}
      </div>

      {activeTab === 'scenarios' && (
        <div style={styles.scenariosSection}>
          <div style={styles.buttonsGrid}>
            <button onClick={() => handleSimulate('price-drop', { position, priceDropPercent: 20, scenarioName: '20% Price Drop' })} disabled={isLoading} style={styles.button}>
              20% Price Drop
            </button>
            <button onClick={() => handleSimulate('rate-increase', { position, rateIncreasePercent: 5, scenarioName: '5% Rate Increase' })} disabled={isLoading} style={styles.button}>
              5% Rate Increase
            </button>
            <button onClick={() => handleSimulate('deposit', { position, depositAmount: 100, scenarioName: 'Add 100 Collateral' })} disabled={isLoading} style={styles.button}>
              Add Collateral
            </button>
            <button onClick={() => handleSimulate('repay', { position, repaymentAmount: 50, scenarioName: 'Repay 50 Debt' })} disabled={isLoading} style={styles.button}>
              Repay Debt
            </button>
          </div>
        </div>
      )}

      {activeTab === 'complex' && (
        <div style={styles.section}>
          <h3>Complex Scenario (Price Drop + Rate Increase)</h3>
          <div style={styles.complexInputs}>
            <div>
              <label style={styles.fieldLabel}>Price Drop %</label>
              <input
                type="number"
                value={complexParams.priceDrop}
                onChange={e => setComplexParams(prev => ({ ...prev, priceDrop: parseFloat(e.target.value) || 0 }))}
                style={styles.input}
              />
            </div>
            <div>
              <label style={styles.fieldLabel}>Rate Increase %</label>
              <input
                type="number"
                value={complexParams.rateIncrease}
                onChange={e => setComplexParams(prev => ({ ...prev, rateIncrease: parseFloat(e.target.value) || 0 }))}
                style={styles.input}
              />
            </div>
            <button
              onClick={() => handleSimulate('complex', {
                position,
                priceDropPercent: complexParams.priceDrop,
                rateIncreasePercent: complexParams.rateIncrease,
                scenarioName: `Complex: ${complexParams.priceDrop}% drop + ${complexParams.rateIncrease}% rate`,
              })}
              disabled={isLoading}
              style={{ ...styles.button, alignSelf: 'flex-end' }}
            >
              Run Complex Simulation
            </button>
          </div>
        </div>
      )}

      {activeTab === 'history' && (
        <div style={styles.section}>
          <h3>Historical Scenario Replay</h3>
          <p style={styles.hint}>Test if your position would have survived historical market events</p>
          <div style={styles.complexInputs}>
            <div>
              <label style={styles.fieldLabel}>Date</label>
              <input
                type="date"
                value={historyDate}
                onChange={e => setHistoryDate(e.target.value)}
                style={styles.input}
              />
            </div>
            <button
              onClick={() => handleSimulate('historical', {
                position,
                historicalDate: historyDate,
                pricesOnDate: { USDC: 0.85 },
                scenarioName: `Historical Replay: ${historyDate}`,
              })}
              disabled={isLoading}
              style={{ ...styles.button, alignSelf: 'flex-end' }}
            >
              Replay Historical
            </button>
          </div>
        </div>
      )}

      {activeTab === 'compare' && (
        <div style={styles.section}>
          <h3>Compare Scenarios</h3>
          <p style={styles.hint}>Select exactly 2 simulation results to compare side-by-side</p>
          <div style={styles.compareList}>
            {simulationResults.map((result, index) => (
              <label key={index} style={styles.compareItem}>
                <input
                  type="checkbox"
                  checked={selectedForComparison.includes(index)}
                  onChange={() => {
                    setSelectedForComparison(prev =>
                      prev.includes(index)
                        ? prev.filter(i => i !== index)
                        : prev.length < 2 ? [...prev, index] : prev
                    );
                  }}
                />
                <span>{result.scenarioName} (HF: {result.afterScenarioHealth.toFixed(2)})</span>
              </label>
            ))}
          </div>
          <button
            onClick={handleCompare}
            disabled={selectedForComparison.length !== 2}
            style={{
              ...styles.button,
              opacity: selectedForComparison.length === 2 ? 1 : 0.5,
              marginTop: '10px',
            }}
          >
            Compare Selected ({selectedForComparison.length}/2)
          </button>
        </div>
      )}

      <div style={styles.section}>
        <h3>Real-time Simulation</h3>
        <div style={styles.complexInputs}>
          <div>
            <label style={styles.fieldLabel}>Action</label>
            <select
              value={realTimeType}
              onChange={e => setRealTimeType(e.target.value as typeof realTimeType)}
              style={styles.input}
            >
              <option value="deposit">Deposit</option>
              <option value="withdraw">Withdraw</option>
              <option value="borrow">Borrow</option>
              <option value="repay">Repay</option>
            </select>
          </div>
          <div>
            <label style={styles.fieldLabel}>Amount</label>
            <input
              type="number"
              value={realTimeAmount}
              onChange={e => {
                setRealTimeAmount(parseFloat(e.target.value) || 0);
              }}
              onBlur={handleRealTimeSimulate}
              style={styles.input}
            />
          </div>
        </div>
        {realTimeResult && (
          <div style={{
            ...styles.resultCard,
            marginTop: '12px',
            borderLeft: `4px solid ${getHealthColor(realTimeResult.afterScenarioHealth)}`,
          }}>
            <div style={styles.resultHeader}>
              <span>{realTimeResult.scenarioName}</span>
              <span style={{
                ...styles.statusBadge,
                backgroundColor: realTimeResult.isLiquidatable ? '#dc3545' : '#28a745',
              }}>
                {realTimeResult.isLiquidatable ? 'AT RISK' : 'SAFE'}
              </span>
            </div>
            <div style={styles.resultMetrics}>
              <p>Current: {realTimeResult.currentHealth.toFixed(2)} → After: {realTimeResult.afterScenarioHealth.toFixed(2)}</p>
              <p>Safety Margin: {realTimeResult.safetyMargin.toFixed(2)}</p>
            </div>
          </div>
        )}
      </div>

      {simulationResults.length > 0 && (
        <div style={styles.resultsSection}>
          <h3>Simulation Results ({simulationResults.length})</h3>
          <button
            onClick={() => setSimulationResults([])}
            style={styles.clearButton}
          >
            Clear All
          </button>
          {simulationResults.map((result, index) => (
            <div key={index} style={{
              ...styles.resultCard,
              borderLeft: `4px solid ${getHealthColor(result.afterScenarioHealth)}`,
            }}>
              <div style={styles.resultHeader}>
                <h4 style={{ margin: 0 }}>{result.scenarioName}</h4>
                <span style={{
                  ...styles.statusBadge,
                  backgroundColor: result.isLiquidatable ? '#dc3545' : '#28a745',
                }}>
                  {result.isLiquidatable ? 'AT RISK' : 'SAFE'}
                </span>
              </div>
              <div style={styles.resultMetrics}>
                <p>Current Health: {result.currentHealth.toFixed(2)}</p>
                <p>After Scenario: {result.afterScenarioHealth.toFixed(2)}</p>
                <p>Safety Margin: {result.safetyMargin.toFixed(2)}</p>
                <p>Liquidation Price: ${result.liquidationPrice.toFixed(2)}</p>
                {result.collateralChange !== undefined && (
                  <p>Collateral Change: {result.collateralChange > 0 ? '+' : ''}{result.collateralChange.toFixed(2)}</p>
                )}
                {result.debtChange !== undefined && (
                  <p>Debt Change: {result.debtChange > 0 ? '+' : ''}{result.debtChange.toFixed(2)}</p>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

const styles: Record<string, React.CSSProperties> = {
  container: {
    padding: '20px',
    maxWidth: '900px',
    margin: '0 auto',
  },
  positionCard: {
    padding: '20px',
    backgroundColor: '#f9f9f9',
    borderRadius: '8px',
    marginBottom: '20px',
    border: '1px solid #e0e0e0',
  },
  positionDetails: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    gap: '15px',
    marginTop: '10px',
  },
  fieldLabel: {
    display: 'block',
    fontSize: '12px',
    fontWeight: 'bold',
    color: '#666',
    marginBottom: '4px',
  },
  input: {
    width: '100%',
    padding: '8px',
    borderRadius: '4px',
    border: '1px solid #ddd',
    fontSize: '14px',
    boxSizing: 'border-box',
  },
  healthFactor: {
    marginTop: '4px',
    padding: '10px',
    borderRadius: '4px',
    textAlign: 'center',
    color: 'white',
    fontWeight: 'bold',
    fontSize: '18px',
  },
  tabBar: {
    display: 'flex',
    gap: '8px',
    marginBottom: '20px',
  },
  tab: {
    padding: '8px 16px',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '13px',
    fontWeight: '600',
  },
  scenariosSection: {
    marginBottom: '20px',
  },
  buttonsGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(2, 1fr)',
    gap: '10px',
  },
  button: {
    padding: '12px',
    backgroundColor: '#007bff',
    color: 'white',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '14px',
    fontWeight: 'bold',
  },
  section: {
    marginBottom: '20px',
    padding: '16px',
    backgroundColor: '#f9f9f9',
    borderRadius: '8px',
    border: '1px solid #e0e0e0',
  },
  complexInputs: {
    display: 'flex',
    gap: '12px',
    alignItems: 'flex-end',
    flexWrap: 'wrap',
  },
  hint: {
    fontSize: '13px',
    color: '#666',
    marginBottom: '12px',
  },
  compareList: {
    display: 'flex',
    flexDirection: 'column',
    gap: '8px',
  },
  compareItem: {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    fontSize: '13px',
    cursor: 'pointer',
  },
  resultsSection: {
    marginTop: '20px',
  },
  clearButton: {
    padding: '6px 12px',
    backgroundColor: '#f0f0f0',
    border: '1px solid #ddd',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '12px',
    marginBottom: '12px',
  },
  resultCard: {
    padding: '15px',
    marginBottom: '10px',
    backgroundColor: '#fff',
    borderRadius: '4px',
    border: '1px solid #e0e0e0',
  },
  resultHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '10px',
  },
  statusBadge: {
    padding: '4px 12px',
    borderRadius: '4px',
    color: 'white',
    fontSize: '12px',
    fontWeight: 'bold',
  },
  resultMetrics: {
    fontSize: '14px',
    lineHeight: '1.6',
  },
};
