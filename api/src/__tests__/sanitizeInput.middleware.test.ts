import { sanitizeInput } from '../middleware/sanitizeInput';

describe('sanitizeInput middleware', () => {
  it('trims nested strings and preserves quotes needed by structured payloads', () => {
    const req = {
      body: {
        data: '  {"source":"legacy"}  ',
        nested: ['  "quoted"  ', { csv: '  "{""kind"":""csv""}"  ' }],
      },
      query: { filter: '  active  ' },
      params: { merchantId: '  merchant_42  ' },
    } as any;
    const res = {} as any;
    const next = jest.fn();

    sanitizeInput(req, res, next);

    expect(req.body).toEqual({
      data: '{"source":"legacy"}',
      nested: ['"quoted"', { csv: '"{""kind"":""csv""}"' }],
    });
    expect(req.query).toEqual({ filter: 'active' });
    expect(req.params).toEqual({ merchantId: 'merchant_42' });
    expect(next).toHaveBeenCalledTimes(1);
  });

  it('removes angle brackets but leaves the rest of the string intact', () => {
    const req = {
      body: { note: '  <script>"keep-quotes"</script>  ' },
      query: {},
      params: {},
    } as any;

    sanitizeInput(req, {} as any, jest.fn());

    expect(req.body.note).toBe('script"keep-quotes"/script');
  });

  it('throws when a string exceeds the maximum allowed length', () => {
    const req = {
      body: { oversized: 'x'.repeat(513) },
      query: {},
      params: {},
    } as any;

    expect(() => sanitizeInput(req, {} as any, jest.fn())).toThrow(
      'Input exceeds maximum length (512)'
    );
  });
});
