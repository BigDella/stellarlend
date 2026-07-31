import swaggerJsdoc from 'swagger-jsdoc';

/**
 * Per-version OpenAPI specification configuration.
 * Each version can have its own info block, servers, and route paths.
 */

export interface VersionedOpenApiConfig {
  version: string;
  title: string;
  description: string;
  routeGlob: string;
  /** Additional glob(s) for route files containing @openapi annotations */
  legacyRouteGlob?: string;
  deprecated?: boolean;
  sunset?: string;
}

const versionConfigs: Record<string, VersionedOpenApiConfig> = {
  v1: {
    version: '1.0.0',
    title: 'StellarLend API v1',
    description: 'REST API v1 for StellarLend core lending operations on Stellar/Soroban',
    routeGlob: './src/routes/v1/**/*.ts',
    // Also scan original route files for @openapi JSDoc annotations
    legacyRouteGlob: './src/routes/*.ts',
  },
};

/**
 * Build an OpenAPI spec for a specific API version.
 */
export function buildVersionedSpec(apiVersion: string): object {
  const vConfig = versionConfigs[apiVersion];
  if (!vConfig) {
    throw new Error(`Unknown API version: ${apiVersion}. Available: ${Object.keys(versionConfigs).join(', ')}`);
  }

  const options: swaggerJsdoc.Options = {
    definition: {
      openapi: '3.0.3',
      info: {
        title: vConfig.title,
        version: vConfig.version,
        description: vConfig.description,
        license: { name: 'MIT' },
      },
      servers: [
        {
          url: `/api/${apiVersion}`,
          description: `${apiVersion.toUpperCase()} base path`,
        },
      ],
    },
    apis: [vConfig.routeGlob, vConfig.legacyRouteGlob].filter((g): g is string => Boolean(g)),
  };

  return swaggerJsdoc(options);
}

/**
 * Returns the current (latest stable) API version.
 */
export function getCurrentVersion(): string {
  return 'v1';
}

/**
 * Build the current/latest OpenAPI spec (legacy compatibility).
 */
export function buildCurrentSpec(): object {
  return buildVersionedSpec(getCurrentVersion());
}

// Legacy swagger spec for backward compatibility
export const swaggerSpec = buildCurrentSpec();

// Version-specific specs
export const v1Spec = buildVersionedSpec('v1');

/**
 * List all available API versions with deprecation status.
 */
export function listVersions(): Array<{ version: string; deprecated: boolean; sunset?: string }> {
  return Object.entries(versionConfigs).map(([version, config]) => ({
    version,
    deprecated: config.deprecated ?? false,
    sunset: config.sunset,
  }));
}

/**
 * Request handler to serve version list.
 */
export function versionListHandler(_req: any, res: any): void {
  res.json({
    versions: listVersions(),
    current: getCurrentVersion(),
  });
}
