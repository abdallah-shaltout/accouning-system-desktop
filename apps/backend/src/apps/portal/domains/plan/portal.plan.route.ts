import { Router } from "express";
import { portalPlanController } from "./portal.plan.controller";

const router = Router();

// Public — no auth. The portal pricing page must work for anonymous visitors.
router.get("/", portalPlanController.listPublished);

export default router;
