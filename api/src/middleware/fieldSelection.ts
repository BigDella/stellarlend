import { Request, Response, NextFunction } from 'express';

function pickFields(
  obj: Record<string, unknown>,
  fields: Set<string>
): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  for (const key of fields) {
    if (Object.prototype.hasOwnProperty.call(obj, key)) {
      result[key] = obj[key];
    }
  }
  return result;
}

function applyFieldSelection(body: unknown, fields: Set<string>): unknown {
  if (!body || typeof body !== 'object' || Array.isArray(body)) {
    return body;
  }
  const obj = body as Record<string, unknown>;
  // Envelope pattern: filter items inside data array, pass meta keys through
  if (Array.isArray(obj.data)) {
    return {
      ...obj,
      data: (obj.data as unknown[]).map((item) =>
        item && typeof item === 'object' && !Array.isArray(item)
          ? pickFields(item as Record<string, unknown>, fields)
          : item
      ),
    };
  }
  return pickFields(obj, fields);
}

export function fieldSelectionMiddleware(
  req: Request,
  res: Response,
  next: NextFunction
): void {
  const rawFields = req.query.fields;
  if (!rawFields || typeof rawFields !== 'string' || rawFields.trim() === '') {
    return next();
  }

  const fields = new Set(
    rawFields
      .split(',')
      .map((f) => f.trim())
      .filter(Boolean)
  );

  if (fields.size === 0) {
    return next();
  }

  const originalJson = res.json.bind(res) as (body?: unknown) => Response;
  (res as unknown as { json: (body?: unknown) => Response }).json = function (
    body?: unknown
  ): Response {
    return originalJson(applyFieldSelection(body, fields));
  };

  next();
}
