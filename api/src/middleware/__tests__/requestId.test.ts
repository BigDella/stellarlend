import { Request, Response, NextFunction } from 'express';
import { requestIdMiddleware } from '../requestId';
import { requestContext } from '../../utils/requestContext';

describe('requestIdMiddleware', () => {
  let mockRequest: Partial<Request>;
  let mockResponse: Partial<Response>;
  let nextFunction: NextFunction;

  beforeEach(() => {
    mockRequest = {
      headers: {},
    };
    mockResponse = {
      setHeader: jest.fn(),
    };
    nextFunction = jest.fn();
  });

  it('should generate a new UUID if x-request-id header is not present', () => {
    requestIdMiddleware(mockRequest as Request, mockResponse as Response, nextFunction);

    expect((mockRequest as any).id).toBeDefined();
    expect(typeof mockRequest.id).toBe('string');
    expect(mockRequest.id?.length).toBeGreaterThan(0);
    expect(mockResponse.setHeader).toHaveBeenCalledWith('x-request-id', mockRequest.id);
    expect(nextFunction).toHaveBeenCalled();
  });

  it('should use incoming x-request-id header if present', () => {
    mockRequest.headers = { 'x-request-id': 'test-req-id' };

    requestIdMiddleware(mockRequest as Request, mockResponse as Response, nextFunction);

    expect(mockRequest.id).toBe('test-req-id');
    expect(mockResponse.setHeader).toHaveBeenCalledWith('x-request-id', 'test-req-id');
    expect(nextFunction).toHaveBeenCalled();
  });

  it('should run next function within async local storage context', (done) => {
    nextFunction = jest.fn(() => {
      try {
        const storeId = requestContext.getStore();
        expect(storeId).toBe(mockRequest.id);
        done();
      } catch (err) {
        done(err);
      }
    });

    requestIdMiddleware(mockRequest as Request, mockResponse as Response, nextFunction);
  });
});
