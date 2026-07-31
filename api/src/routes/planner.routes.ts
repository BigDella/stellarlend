import { Router } from 'express';
import { budgetPlanner } from '../services/planner/budget-planner';
const router = Router();
router.post('/budget', (req, res) => {
  try {
    res.json({ success: true, data: budgetPlanner.build(req.body) });
  } catch (error) {
    res
      .status(400)
      .json({ success: false, error: error instanceof Error ? error.message : 'Invalid plan' });
  }
});
export default router;
