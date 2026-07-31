import { StressScenario } from './types';
import { PREDEFINED_SCENARIOS, CUSTOM_SCENARIO_TEMPLATE } from './scenario-library';

class ScenarioRegistry {
  private scenarios: Map<string, StressScenario> = new Map();
  private customCount = 0;

  constructor() {
    for (const scenario of PREDEFINED_SCENARIOS) {
      this.scenarios.set(scenario.id, scenario);
    }
  }

  getScenario(id: string): StressScenario | undefined {
    return this.scenarios.get(id);
  }

  getAllScenarios(): StressScenario[] {
    return Array.from(this.scenarios.values());
  }

  getScenariosByCategory(category: string): StressScenario[] {
    return this.getAllScenarios().filter((s) => s.category === category);
  }

  getScenariosByTag(tag: string): StressScenario[] {
    return this.getAllScenarios().filter((s) => s.tags.includes(tag));
  }

  addCustomScenario(scenario: StressScenario): StressScenario {
    const existing = this.scenarios.get(scenario.id);
    if (existing) {
      const newVersion = this.bumpVersion(existing.version);
      const newScenario: StressScenario = {
        ...scenario,
        id: `${scenario.id}-v${newVersion}`,
        version: newVersion,
        created: new Date().toISOString(),
      };
      this.scenarios.set(newScenario.id, newScenario);
      return newScenario;
    }

    this.scenarios.set(scenario.id, scenario);
    return scenario;
  }

  buildCustomScenario(config: {
    name: string;
    description: string;
    priceChanges: StressScenario['priceChanges'];
    correlationShifts?: StressScenario['correlationShifts'];
    volatilityMultipliers?: StressScenario['volatilityMultipliers'];
    tags?: string[];
  }): StressScenario {
    this.customCount++;
    const id = `custom-${Date.now()}-${this.customCount}`;

    return {
      ...CUSTOM_SCENARIO_TEMPLATE,
      id,
      name: config.name,
      description: config.description,
      priceChanges: config.priceChanges,
      correlationShifts: config.correlationShifts ?? [],
      volatilityMultipliers: config.volatilityMultipliers ?? [],
      tags: config.tags ?? ['custom'],
      created: new Date().toISOString(),
    };
  }

  removeCustomScenario(id: string): boolean {
    const scenario = this.scenarios.get(id);
    if (scenario && scenario.category === 'custom') {
      this.scenarios.delete(id);
      return true;
    }
    return false;
  }

  exportScenario(id: string): string | null {
    const scenario = this.scenarios.get(id);
    if (!scenario) return null;
    return JSON.stringify(scenario, null, 2);
  }

  importScenario(json: string): StressScenario | null {
    try {
      const scenario: StressScenario = JSON.parse(json);
      if (this.validateScenario(scenario)) {
        this.addCustomScenario(scenario);
        return scenario;
      }
      return null;
    } catch {
      return null;
    }
  }

  private validateScenario(scenario: StressScenario): boolean {
    if (!scenario.id || !scenario.name) return false;
    if (!Array.isArray(scenario.priceChanges)) return false;
    if (!Array.isArray(scenario.tags)) return false;
    if (!['historical', 'custom', 'hypothetical'].includes(scenario.category)) return false;
    return true;
  }

  private bumpVersion(version: string): string {
    const parts = version.split('.');
    const patch = parseInt(parts[2] || '0') + 1;
    parts[2] = patch.toString();
    return parts.join('.');
  }
}

export const scenarioRegistry = new ScenarioRegistry();
