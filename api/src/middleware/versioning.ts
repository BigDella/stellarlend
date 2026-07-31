/**
 * Version Migration & Deprecation Middleware
 *
 * Enables smooth API version transitions:
 * - Adds deprecation warning headers to legacy route paths
 * - Provides version aliasing for clients that haven't migrated
 * - Allows route-level version pinning
 *
 * Usage:
 *   app.use('/api/legacy-path', versionMiddleware({ deprecatedIn: 'v2', sunset: '2026-12-31' }), legacyRoutes);
 *   app.use('/api/v1/path', versionMiddleware({ version: 'v1' }), v1Routes);
 */

import { Request, Response, NextFunction } from 'express';

/**
 * Augment Express Response to add version helpers.
 */
interface VersionedResponse extends Response {
  addDeprecation?: (config: {
    deprecatedIn: string;
    sunset?: string;
    migrateTo?: string;
  }) => void;
}

export interface VersionConfig {
  /** Current API version for this route prefix */
  version?: string;
  /** Version in which this route was deprecated */
  deprecatedIn?: string;
  /** ISO date string after which this version will be removed */
  sunset?: string;
  /** Alternative v1 path clients should migrate to */
  migrateTo?: string;
}

/**
 * Express middleware that adds version and deprecation headers.
 * Also patches `res.json` to inject deprecation metadata into the response body.
 */
export function versionMiddleware(config: VersionConfig) {
  return (req: Request, res: VersionedResponse, next: NextFunction): void => {
    // Attach version info to response headers
    if (config.version) {
      res.setHeader('X-API-Version', config.version);
    }

    // Add deprecation headers for legacy routes
    if (config.deprecatedIn) {
      res.setHeader('X-API-Deprecated', 'true');
      res.setHeader('X-API-Deprecated-In', config.deprecatedIn);

      if (config.sunset) {
        res.setHeader('X-API-Sunset', config.sunset);
      }

      if (config.migrateTo) {
        res.setHeader('X-API-Migrate-To', config.migrateTo);
        // Also add a Link header for machine-readable migration
        res.setHeader(
          'Link',
          `<${config.migrateTo}>; rel="alternate"; title="Latest version"`
        );
      }

      // Inject deprecation metadata into JSON response body
      // so in-band consumers (e.g., SDKs, curl users) see the migration notice
      const originalJson = res.json.bind(res);
      res.json = function (body: any): Response {
        const deprecationMeta: Record<string, unknown> = {
          deprecated: true,
          deprecatedIn: config.deprecatedIn,
        };
        if (config.sunset) deprecationMeta.sunset = config.sunset;
        if (config.migrateTo) deprecationMeta.migrateTo = config.migrateTo;

        if (typeof body === 'object' && body !== null && !Array.isArray(body)) {
          body.deprecation = deprecationMeta;
          body._links = {
            ...(body._links || {}),
            latestVersion: { href: config.migrateTo },
          };
        }

        return originalJson(body);
      };
    }

    next();
  };
}

/**
 * Creates a combined middleware stack that adds deprecation headers
 * and runs version middleware for legacy route compatibility.
 *
 * Legacy routes are mounted at their old paths with deprecation warnings,
 * while new v1 routes are served under /api/v1/*.
 */
export function legacyCompatibilityMiddleware(migrateToPath: string) {
  return versionMiddleware({
    deprecatedIn: 'v2',
    sunset: '2027-06-30',
    migrateTo: migrateToPath,
  });
}
