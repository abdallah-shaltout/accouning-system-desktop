import { Router } from "express";
import adminRoutes from "@/admin/routes";
import portalRoutes from "@/portal/routes";
import deviceRoutes from "@/device/routes";

const router = Router();

router.use("/admin", adminRoutes);
router.use("/portal", portalRoutes);
router.use("/device", deviceRoutes);

export default router;
