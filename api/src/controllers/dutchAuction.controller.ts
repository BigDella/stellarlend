import { Request, Response, NextFunction } from 'express';
import { dutchAuctionService } from '../services/dutchAuction.service';

export class DutchAuctionController {
  async getAuctions(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const { filter, sortBy, sortDir } = req.query;
      const auctions = await dutchAuctionService.getAuctions(
        filter as string,
        sortBy as string,
        sortDir as string
      );
      res.status(200).json({ auctions, total: auctions.length });
    } catch (err) {
      next(err);
    }
  }

  async getAnalytics(_req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const analytics = await dutchAuctionService.getAnalytics();
      res.status(200).json(analytics);
    } catch (err) {
      next(err);
    }
  }

  async placeBid(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const { auctionId, bidder, amount } = req.body;
      if (!auctionId || !bidder || !amount) {
        res.status(400).json({ error: 'auctionId, bidder, and amount are required' });
        return;
      }
      await dutchAuctionService.placeBid(auctionId, bidder, amount);
      res.status(200).json({ message: 'Bid placed successfully', auctionId });
    } catch (err) {
      next(err);
    }
  }
}

export const dutchAuctionController = new DutchAuctionController();