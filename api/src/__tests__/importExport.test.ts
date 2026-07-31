import { exportSubscriptions, importSubscriptions, validateImport } from '../utils/importExport';
import { ImportData, SubscriptionRecord } from '../types';

const EXISTING_SUBSCRIPTION: SubscriptionRecord = {
  merchantId: 'merchant_1',
  subscriptionId: 'sub_existing',
  customerEmail: 'existing@example.com',
  planId: 'starter',
  status: 'active',
  amount: 25,
  currency: 'USD',
  interval: 'monthly',
  startDate: '2026-01-01T00:00:00.000Z',
  nextBillingDate: '2026-02-01T00:00:00.000Z',
  metadata: { source: 'seed' },
  createdAt: '2026-01-01T00:00:00.000Z',
  updatedAt: '2026-01-01T00:00:00.000Z',
};

describe('importExport utilities', () => {
  it('validates CSV imports with column mapping and preview actions', () => {
    const payload: ImportData = {
      merchantId: 'merchant_1',
      format: 'csv',
      data: [
        'id,email,plan,state,price,currency,billing,start_date,next_billing,metadata_json',
        'sub_new,new@example.com,pro,active,49.5,usd,monthly,2026-03-01,2026-04-01,"{""source"":""csv""}"',
      ].join('\n'),
      columnMapping: {
        id: 'subscriptionId',
        email: 'customerEmail',
        plan: 'planId',
        state: 'status',
        price: 'amount',
        billing: 'interval',
        start_date: 'startDate',
        next_billing: 'nextBillingDate',
        metadata_json: 'metadata',
      },
    };

    const result = validateImport(payload, [EXISTING_SUBSCRIPTION]);

    expect(result.isValid).toBe(true);
    expect(result.summary.totalRows).toBe(1);
    expect(result.summary.creates).toBe(1);
    expect(result.previewRows[0]).toMatchObject({
      rowNumber: 2,
      action: 'create',
      subscription: {
        subscriptionId: 'sub_new',
        customerEmail: 'new@example.com',
        amount: 49.5,
        currency: 'USD',
      },
    });
  });

  it('reports row-level validation and duplicate errors', () => {
    const payload: ImportData = {
      merchantId: 'merchant_1',
      format: 'json',
      data: [
        {
          subscriptionId: 'sub_dup',
          customerEmail: 'bad-email',
          planId: 'starter',
          status: 'broken',
          amount: 20,
          currency: 'usd',
          interval: 'monthly',
          startDate: '2026-01-01',
        },
        {
          subscriptionId: 'sub_dup',
          customerEmail: 'good@example.com',
          planId: 'starter',
          status: 'active',
          amount: 20,
          currency: 'usd',
          interval: 'monthly',
          startDate: '2026-01-01',
        },
      ],
    };

    const result = validateImport(payload);

    expect(result.isValid).toBe(false);
    expect(result.summary.invalidRows).toBe(1);
    expect(result.errors).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          rowNumber: 1,
          field: 'customerEmail',
        }),
        expect.objectContaining({
          rowNumber: 1,
          field: 'status',
        }),
      ])
    );
  });

  it('supports incremental upsert and skips unchanged subscriptions', () => {
    const payload: ImportData = {
      merchantId: 'merchant_1',
      format: 'json',
      data: [
        {
          subscriptionId: 'sub_existing',
          customerEmail: 'existing@example.com',
          planId: 'starter',
          status: 'active',
          amount: 30,
          currency: 'usd',
          interval: 'monthly',
          startDate: '2026-01-01',
          nextBillingDate: '2026-02-01',
          metadata: { source: 'seed' },
        },
        {
          subscriptionId: 'sub_same',
          customerEmail: 'same@example.com',
          planId: 'starter',
          status: 'active',
          amount: 15,
          currency: 'usd',
          interval: 'monthly',
          startDate: '2026-01-01',
        },
      ],
    };

    const existing: SubscriptionRecord[] = [
      EXISTING_SUBSCRIPTION,
      {
        ...EXISTING_SUBSCRIPTION,
        subscriptionId: 'sub_same',
        customerEmail: 'same@example.com',
        amount: 15,
        startDate: '2026-01-01T00:00:00.000Z',
        nextBillingDate: undefined,
        metadata: undefined,
      },
    ];

    const result = importSubscriptions(payload, existing);

    expect(result.updatedCount).toBe(1);
    expect(result.skippedCount).toBe(1);
    expect(
      result.appliedSubscriptions.find((item) => item.subscriptionId === 'sub_existing')?.amount
    ).toBe(30);
  });

  it('exports deterministic JSON subscription data', () => {
    const result = exportSubscriptions('merchant_1', [
      { ...EXISTING_SUBSCRIPTION, subscriptionId: 'sub_b' },
      { ...EXISTING_SUBSCRIPTION, subscriptionId: 'sub_a' },
    ]);

    expect(result.format).toBe('json');
    expect(result.count).toBe(2);
    expect(result.subscriptions.map((item) => item.subscriptionId)).toEqual(['sub_a', 'sub_b']);
  });

  it('rejects very large imports', () => {
    const rows = Array.from({ length: 5001 }, (_, index) => ({
      subscriptionId: `sub_${index}`,
      customerEmail: `user${index}@example.com`,
      planId: 'starter',
      status: 'active',
      amount: 10,
      currency: 'usd',
      interval: 'monthly',
      startDate: '2026-01-01',
    }));

    expect(() =>
      validateImport({
        merchantId: 'merchant_1',
        format: 'json',
        data: rows,
      })
    ).toThrow('Import payload exceeds the maximum of 5000 rows');
  });
});
