import { Request, Response, NextFunction } from 'express';
import jwt from 'jsonwebtoken';
import { authenticateToken, AuthRequest } from '../middleware/auth';
import { UnauthorizedError } from '../utils/errors';

// Mock the config module
jest.mock('../config', () => ({
  config: {
    auth: {
      jwtSecret: 'test-secret-key-for-testing',
      jwtExpiresIn: '1h',
    },
    logging: {
      level: 'silent',
    },
  },
}));

describe('Auth Middleware', () => {
  let mockRequest: Partial<AuthRequest>;
  let mockResponse: Partial<Response>;
  let mockNext: NextFunction;

  beforeEach(() => {
    mockRequest = {
      headers: {},
    };
    mockResponse = {};
    mockNext = jest.fn();
  });

  describe('authenticateToken', () => {
    it('should pass through with valid token', () => {
      const validToken = jwt.sign(
        { address: '0x1234567890123456789012345678901234567890' },
        'test-secret-key-for-testing'
      );
      mockRequest.headers = {
        authorization: `Bearer ${validToken}`,
      };

      authenticateToken(mockRequest as AuthRequest, mockResponse as Response, mockNext);

      expect(mockNext).toHaveBeenCalledWith();
      expect(mockRequest.user).toMatchObject({
        address: '0x1234567890123456789012345678901234567890',
      });
    });

    it('should return 401 when token is missing', () => {
      mockRequest.headers = {};

      expect(() => {
        authenticateToken(mockRequest as AuthRequest, mockResponse as Response, mockNext);
      }).toThrow(UnauthorizedError);

      expect(mockNext).not.toHaveBeenCalled();
    });

    it('should return 401 when token is expired', () => {
      const expiredToken = jwt.sign(
        { address: '0x1234567890123456789012345678901234567890' },
        'test-secret-key-for-testing',
        { expiresIn: '-1s' }
      );
      mockRequest.headers = {
        authorization: `Bearer ${expiredToken}`,
      };

      expect(() => {
        authenticateToken(mockRequest as AuthRequest, mockResponse as Response, mockNext);
      }).toThrow(UnauthorizedError);

      expect(mockNext).not.toHaveBeenCalled();
    });

    it('should return 401 when token is invalid', () => {
      const invalidToken = 'invalid-token-string';
      mockRequest.headers = {
        authorization: `Bearer ${invalidToken}`,
      };

      expect(() => {
        authenticateToken(mockRequest as AuthRequest, mockResponse as Response, mockNext);
      }).toThrow(UnauthorizedError);

      expect(mockNext).not.toHaveBeenCalled();
    });

    it('should return 401 when authorization header is malformed', () => {
      mockRequest.headers = {
        authorization: 'InvalidFormat token',
      };

      expect(() => {
        authenticateToken(mockRequest as AuthRequest, mockResponse as Response, mockNext);
      }).toThrow(UnauthorizedError);

      expect(mockNext).not.toHaveBeenCalled();
    });

    it('should return 401 when authorization header has no token', () => {
      mockRequest.headers = {
        authorization: 'Bearer',
      };

      expect(() => {
        authenticateToken(mockRequest as AuthRequest, mockResponse as Response, mockNext);
      }).toThrow(UnauthorizedError);

      expect(mockNext).not.toHaveBeenCalled();
    });

    it('should return 401 when authorization header has only Bearer without space', () => {
      mockRequest.headers = {
        authorization: 'Bearertoken',
      };

      expect(() => {
        authenticateToken(mockRequest as AuthRequest, mockResponse as Response, mockNext);
      }).toThrow(UnauthorizedError);

      expect(mockNext).not.toHaveBeenCalled();
    });
  });
});
