import { Router } from 'express';
import { dutchAuctionController } from '../controllers/dutchAuction.controller';

const router = Router();

router.get('/', dutchAuctionController.getAuctions);
router.get('/analytics', dutchAuctionController.getAnalytics);
router.post('/bid', dutchAuctionController.placeBid);

export default router;