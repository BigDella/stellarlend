'use strict';

/**
 * StellarLend Mock Oracle
 *
 * Serves simulated asset prices for local development.
 * Prices drift slightly on each update to simulate real market movement.
 *
 * Endpoints:
 *   GET /health          – liveness probe
 *   GET /prices          – all asset prices
 *   GET /prices/:asset   – single asset price
 */

const http = require('http');

const PORT = parseInt(process.env.PORT || '4000', 10);
const UPDATE_INTERVAL_MS = parseInt(process.env.UPDATE_INTERVAL_MS || '5000', 10);

// Base prices in USD (7 decimal places, Stellar stroops convention)
const prices = {
  XLM:  { price: '0.1100000', asset: 'XLM',  updatedAt: Date.now() },
  USDC: { price: '1.0000000', asset: 'USDC', updatedAt: Date.now() },
  BTC:  { price: '67000.0000000', asset: 'BTC',  updatedAt: Date.now() },
  ETH:  { price: '3500.0000000',  asset: 'ETH',  updatedAt: Date.now() },
};

/** Apply a small random drift to simulate live prices */
function driftPrices() {
  for (const key of Object.keys(prices)) {
    if (key === 'USDC') continue; // stablecoin stays pegged
    const current = parseFloat(prices[key].price);
    const drift = current * (Math.random() * 0.004 - 0.002); // ±0.2%
    prices[key].price = Math.max(0, current + drift).toFixed(7);
    prices[key].updatedAt = Date.now();
  }
}

setInterval(driftPrices, UPDATE_INTERVAL_MS);

function sendJson(res, statusCode, body) {
  const payload = JSON.stringify(body);
  res.writeHead(statusCode, {
    'Content-Type': 'application/json',
    'Content-Length': Buffer.byteLength(payload),
  });
  res.end(payload);
}

const server = http.createServer((req, res) => {
  const url = req.url || '/';

  if (url === '/health') {
    return sendJson(res, 200, { status: 'ok', service: 'stellarlend-oracle' });
  }

  if (url === '/prices') {
    return sendJson(res, 200, { success: true, data: prices });
  }

  const assetMatch = url.match(/^\/prices\/([A-Z]+)$/);
  if (assetMatch) {
    const asset = assetMatch[1].toUpperCase();
    if (prices[asset]) {
      return sendJson(res, 200, { success: true, data: prices[asset] });
    }
    return sendJson(res, 404, { success: false, error: `Asset ${asset} not found` });
  }

  sendJson(res, 404, { success: false, error: 'Not found' });
});

server.listen(PORT, () => {
  console.log(`[oracle] Mock price oracle listening on port ${PORT}`);
  console.log(`[oracle] Price update interval: ${UPDATE_INTERVAL_MS}ms`);
});
