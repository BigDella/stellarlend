import request from 'supertest';
import app from '../app';
import { resetSubscriptionStore } from '../services/subscription.service';

describe('Subscription import/export routes', () => {
  beforeEach(() => {
    resetSubscriptionStore();
  });

  it('provides preview, import, export, upsert, and history flows', async () => {
    const csvPayload = {
      merchantId: 'merchant_42',
      format: 'csv',
      data: [
        'legacy_id,subscriber_email,plan_code,status,amount,currency,interval,start_date,next_billing_date,metadata',
        'sub_001,ada@example.com,growth,active,29.99,usd,monthly,2026-03-01,2026-04-01,"{""source"":""legacy""}"',
      ].join('\n'),
      columnMapping: {
        legacy_id: 'subscriptionId',
        subscriber_email: 'customerEmail',
        plan_code: 'planId',
        start_date: 'startDate',
        next_billing_date: 'nextBillingDate',
      },
      options: {
        upsert: true,
        previewLimit: 10,
      },
    };

    const previewResponse = await request(app)
      .post('/api/subscriptions/import/preview')
      .send(csvPayload);

    expect(previewResponse.status).toBe(200);
    expect(previewResponse.body.isValid).toBe(true);
    expect(previewResponse.body.previewRows).toEqual([
      expect.objectContaining({
        rowNumber: 2,
        action: 'create',
        subscription: expect.objectContaining({
          subscriptionId: 'sub_001',
          customerEmail: 'ada@example.com',
        }),
      }),
    ]);

    const importResponse = await request(app).post('/api/subscriptions/import').send(csvPayload);

    expect(importResponse.status).toBe(200);
    expect(importResponse.body.success).toBe(true);
    expect(importResponse.body.importedCount).toBe(1);
    expect(importResponse.body.updatedCount).toBe(0);

    const exportResponse = await request(app).get('/api/subscriptions/export/merchant_42');

    expect(exportResponse.status).toBe(200);
    expect(exportResponse.body).toMatchObject({
      merchantId: 'merchant_42',
      format: 'json',
      count: 1,
    });
    expect(exportResponse.body.subscriptions[0]).toMatchObject({
      subscriptionId: 'sub_001',
      customerEmail: 'ada@example.com',
      amount: 29.99,
      currency: 'USD',
    });

    const upsertResponse = await request(app)
      .post('/api/subscriptions/import')
      .send({
        merchantId: 'merchant_42',
        format: 'json',
        data: [
          {
            subscriptionId: 'sub_001',
            customerEmail: 'ada@example.com',
            planId: 'growth',
            status: 'paused',
            amount: 35,
            currency: 'usd',
            interval: 'monthly',
            startDate: '2026-03-01',
            nextBillingDate: '2026-04-15',
            metadata: { source: 'json-upsert' },
          },
        ],
        options: {
          upsert: true,
        },
      });

    expect(upsertResponse.status).toBe(200);
    expect(upsertResponse.body.updatedCount).toBe(1);

    const historyResponse = await request(app).get('/api/subscriptions/import/history/merchant_42');

    expect(historyResponse.status).toBe(200);
    expect(historyResponse.body.history).toHaveLength(2);
    expect(historyResponse.body.history[0]).toMatchObject({
      merchantId: 'merchant_42',
      status: 'completed',
    });
  });

  it('returns detailed validation errors for invalid imports', async () => {
    const response = await request(app)
      .post('/api/subscriptions/import')
      .send({
        merchantId: 'merchant_42',
        format: 'json',
        data: [
          {
            subscriptionId: 'sub_invalid',
            customerEmail: 'not-an-email',
            planId: '',
            status: 'broken',
            amount: -1,
            currency: 'usdt',
            interval: 'monthly',
            startDate: 'bad-date',
          },
        ],
      });

    expect(response.status).toBe(400);
    expect(response.body.success).toBe(false);
    expect(response.body.errorCount).toBeGreaterThan(0);
    expect(response.body.errors).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ field: 'customerEmail' }),
        expect.objectContaining({ field: 'planId' }),
        expect.objectContaining({ field: 'status' }),
        expect.objectContaining({ field: 'amount' }),
        expect.objectContaining({ field: 'currency' }),
        expect.objectContaining({ field: 'startDate' }),
      ])
    );
  });

  it('rejects malformed JSON import strings during preview', async () => {
    const response = await request(app).post('/api/subscriptions/import/preview').send({
      merchantId: 'merchant_42',
      format: 'json',
      data: '[{"subscriptionId":"sub_1"}',
    });

    expect(response.status).toBe(400);
    expect(response.body.error.message).toBe('Invalid JSON import payload');
  });
});
