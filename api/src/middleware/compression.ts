import { Request, Response, NextFunction } from 'express';
import zlib from 'zlib';
import logger from '../utils/logger';

interface CompressionConfig {
  level: number; // 1-11 for brotli
  minSize: number; // Minimum response size in bytes
  excludeContentTypes: string[];
}

const defaultConfig: CompressionConfig = {
  level: 6, // Balanced compression
  minSize: 1024, // 1KB minimum
  excludeContentTypes: [
    'image/',
    'video/',
    'audio/',
    'application/zip',
    'application/gzip',
    'application/x-brotli',
  ],
};

export function compressionMiddleware(config: Partial<CompressionConfig> = {}) {
  const finalConfig = { ...defaultConfig, ...config };

  return (req: Request, res: Response, next: NextFunction) => {
    const acceptEncoding = req.headers['accept-encoding'] || '';
    const originalSend = res.send.bind(res);
    const originalJson = res.json.bind(res);

    // Override res.send
    res.send = function (body: any): Response {
      return compressAndSend(this, body, originalSend, acceptEncoding, finalConfig);
    };

    // Override res.json
    res.json = function (body: any): Response {
      const jsonString = JSON.stringify(body);
      return compressAndSend(this, jsonString, originalSend, acceptEncoding, finalConfig);
    };

    next();
  };
}

function compressAndSend(
  res: Response,
  body: any,
  originalSend: (body: any) => Response,
  acceptEncoding: string,
  config: CompressionConfig
): Response {
  // Skip if already sent
  if (res.headersSent) {
    return originalSend(body);
  }

  // Get content type
  const contentType = (res.getHeader('content-type') as string) || '';

  // Skip compression for excluded content types
  if (config.excludeContentTypes.some((type) => contentType.startsWith(type))) {
    return originalSend(body);
  }

  // Skip if already compressed
  if (res.getHeader('content-encoding')) {
    return originalSend(body);
  }

  // Convert body to buffer
  let buffer: Buffer;
  if (Buffer.isBuffer(body)) {
    buffer = body;
  } else if (typeof body === 'string') {
    buffer = Buffer.from(body, 'utf-8');
  } else {
    buffer = Buffer.from(JSON.stringify(body), 'utf-8');
  }

  // Skip if below minimum size
  if (buffer.length < config.minSize) {
    return originalSend(body);
  }

  // Preserve cache-control headers
  const cacheControl = res.getHeader('cache-control');

  // Determine compression method
  if (acceptEncoding.includes('br')) {
    // Use Brotli
    const compressed = zlib.brotliCompressSync(buffer, {
      params: {
        [zlib.constants.BROTLI_PARAM_QUALITY]: config.level,
        [zlib.constants.BROTLI_PARAM_MODE]: zlib.constants.BROTLI_MODE_TEXT,
      },
    });

    const compressionRatio = ((1 - compressed.length / buffer.length) * 100).toFixed(2);

    res.setHeader('Content-Encoding', 'br');
    res.setHeader('Content-Length', compressed.length.toString());
    res.setHeader('X-Compression-Ratio', compressionRatio);

    if (cacheControl) {
      res.setHeader('Cache-Control', cacheControl);
    }

    logger.debug('Brotli compression applied', {
      originalSize: buffer.length,
      compressedSize: compressed.length,
      ratio: `${compressionRatio}%`,
    });

    return originalSend(compressed);
  } else if (acceptEncoding.includes('gzip')) {
    // Fallback to Gzip
    const compressed = zlib.gzipSync(buffer, {
      level: Math.min(config.level, 9), // Gzip max level is 9
    });

    const compressionRatio = ((1 - compressed.length / buffer.length) * 100).toFixed(2);

    res.setHeader('Content-Encoding', 'gzip');
    res.setHeader('Content-Length', compressed.length.toString());
    res.setHeader('X-Compression-Ratio', compressionRatio);

    if (cacheControl) {
      res.setHeader('Cache-Control', cacheControl);
    }

    logger.debug('Gzip compression applied', {
      originalSize: buffer.length,
      compressedSize: compressed.length,
      ratio: `${compressionRatio}%`,
    });

    return originalSend(compressed);
  }

  // No compression support
  return originalSend(body);
}

// Streaming compression for large responses
export function streamCompressionMiddleware(config: Partial<CompressionConfig> = {}) {
  const finalConfig = { ...defaultConfig, ...config };

  return (req: Request, res: Response, next: NextFunction) => {
    const acceptEncoding = req.headers['accept-encoding'] || '';
    const originalWrite = res.write.bind(res);
    const originalEnd = res.end.bind(res);

    let compressionStream: zlib.BrotliCompress | zlib.Gzip | null = null;
    let isCompressing = false;

    // Override res.write
    res.write = function (chunk: any, encoding?: any, callback?: any): boolean {
      if (!isCompressing && !res.headersSent) {
        setupCompression();
      }

      if (compressionStream) {
        return compressionStream.write(chunk, encoding, callback);
      }

      return originalWrite(chunk, encoding, callback);
    };

    // Override res.end
    res.end = function (chunk?: any, encoding?: any, callback?: any): Response {
      if (!isCompressing && !res.headersSent) {
        setupCompression();
      }

      if (compressionStream) {
        compressionStream.end(chunk, encoding, callback);
        return res;
      }

      return originalEnd(chunk, encoding, callback);
    };

    function setupCompression() {
      const contentType = (res.getHeader('content-type') as string) || '';

      // Skip compression for excluded types
      if (finalConfig.excludeContentTypes.some((type) => contentType.startsWith(type))) {
        return;
      }

      // Skip if already compressed
      if (res.getHeader('content-encoding')) {
        return;
      }

      isCompressing = true;

      if (acceptEncoding.includes('br')) {
        compressionStream = zlib.createBrotliCompress({
          params: {
            [zlib.constants.BROTLI_PARAM_QUALITY]: finalConfig.level,
          },
        });
        res.setHeader('Content-Encoding', 'br');
      } else if (acceptEncoding.includes('gzip')) {
        compressionStream = zlib.createGzip({
          level: Math.min(finalConfig.level, 9),
        });
        res.setHeader('Content-Encoding', 'gzip');
      }

      if (compressionStream) {
        res.removeHeader('Content-Length');
        compressionStream.pipe(res as any);
      }
    }

    next();
  };
}
