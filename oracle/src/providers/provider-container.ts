import type { HealthStatus, RawPriceData } from '../types/index.js';
import { BasePriceProvider } from './base-provider.js';

export interface PriceProvider {
  readonly name: string;
  readonly priority: number;
  readonly weight: number;
  readonly isEnabled: boolean;
  fetchPrice(asset: string): Promise<RawPriceData>;
  fetchPrices(assets: string[]): Promise<RawPriceData[]>;
  healthCheck(): Promise<HealthStatus>;
}

export type ProviderFactory = () => PriceProvider;

export class ProviderContainer {
  private readonly factories = new Map<string, ProviderFactory>();

  register(name: string, factory: ProviderFactory): this {
    if (this.factories.has(name)) {
      throw new Error(`Provider already registered: ${name}`);
    }
    this.factories.set(name, factory);
    return this;
  }

  resolve(name: string): PriceProvider {
    const factory = this.factories.get(name);
    if (!factory) {
      throw new Error(`Provider not registered: ${name}`);
    }
    return factory();
  }

  resolveChain(names: string[]): PriceProvider[] {
    return names
      .map((name) => this.resolve(name))
      .filter((provider) => provider.isEnabled)
      .sort((a, b) => a.priority - b.priority);
  }

  async healthCheck(names: string[]): Promise<HealthStatus[]> {
    const providers = this.resolveChain(names);
    return Promise.all(providers.map((provider) => provider.healthCheck()));
  }
}

export function createProviderContainer(): ProviderContainer {
  return new ProviderContainer();
}

export function asPriceProvider(provider: BasePriceProvider): PriceProvider {
  return provider;
}
